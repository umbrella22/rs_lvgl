//! Application-owned inputs for the vendored native provider.

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use bindgen::callbacks::{IntKind, MacroParsingBehavior, ParseCallbacks};

#[derive(Default)]
pub struct Inputs {
    pub config: Option<PathBuf>,
    pub extra_c: Option<PathBuf>,
    pub headers: Vec<PathBuf>,
}

impl Inputs {
    pub fn from_env() -> Self {
        for name in ["LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS"] {
            println!("cargo:rerun-if-env-changed={name}");
        }
        let one = |name: &str, directory| {
            env::var_os(name).map(|value| {
                checked_path(name, PathBuf::from(value), directory)
                    .unwrap_or_else(|error| panic!("{error}"))
            })
        };
        let headers = env::var_os("LVGL_EXTRA_HEADERS")
            .map(|value| {
                env::split_paths(&value)
                    .map(|path| {
                        checked_path("LVGL_EXTRA_HEADERS", path, false)
                            .unwrap_or_else(|error| panic!("{error}"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let result = Self {
            config: one("LVGL_CONFIG_PATH", false),
            extra_c: one("LVGL_EXTRA_C_DIR", true),
            headers,
        };
        for path in result
            .config
            .iter()
            .chain(result.extra_c.iter())
            .chain(&result.headers)
        {
            // A directory watch also detects added/removed C files and private
            // headers that are not reachable from the public bindgen headers.
            println!("cargo:rerun-if-changed={}", path.display());
        }
        result
    }

    pub fn include_dirs(&self) -> Vec<&Path> {
        self.config
            .iter()
            .filter_map(|p| p.parent())
            .chain(self.extra_c.iter().map(PathBuf::as_path))
            .chain(self.headers.iter().filter_map(|p| p.parent()))
            .collect()
    }
}

pub fn checked_path(name: &str, path: PathBuf, directory: bool) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!(
            "{name} must be an absolute path; use Cargo [env] relative = true"
        ));
    }
    if (directory && !path.is_dir()) || (!directory && !path.is_file()) {
        return Err(format!(
            "{name} must name an existing {}",
            if directory { "directory" } else { "file" }
        ));
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("{name}: {error}"))?;
    include_directive(&path)?;
    Ok(path)
}

pub fn include_directive(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or("native header paths must be valid UTF-8")?;
    if text.contains(['"', '\n', '\r']) {
        return Err("native header path cannot be represented in a C include directive".into());
    }
    Ok(format!("#include \"{text}\"\n"))
}

/// Match only the explicitly supplied header in bindgen's file allowlist.
pub fn file_pattern(path: &Path) -> String {
    let mut out = String::from("^");
    for ch in path.to_str().expect("validated header path").chars() {
        if "\\.^$|?*+()[]{}".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('$');
    out
}

fn is_source_header(name: &str) -> bool {
    // Cargo compares watched inputs to the start of the build script. Watching
    // a header generated during that script makes every subsequent build dirty.
    // The inputs which create these projections are tracked separately.
    env::var_os("OUT_DIR").is_none_or(|out| !Path::new(name).starts_with(out))
}

#[derive(Clone, Debug, Default)]
pub struct MacroValues(Arc<Mutex<BTreeMap<String, i64>>>);

impl MacroValues {
    pub fn names(&self) -> Vec<String> {
        self.0
            .lock()
            .expect("macro values")
            .keys()
            .cloned()
            .collect()
    }

    pub fn read_constants(&self, bindings: &str) {
        let mut values = self.0.lock().expect("macro values");
        values.clear();
        for line in bindings.lines() {
            let Some(rest) = line.strip_prefix("pub const LV_") else {
                continue;
            };
            let Some((name, value)) = rest.split_once(": ") else {
                continue;
            };
            let Some((_, value)) = value.split_once(" = ") else {
                continue;
            };
            if let Ok(value) = value.trim_end_matches(';').parse::<i64>() {
                values.insert(format!("LV_{name}"), value);
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<i64> {
        self.0.lock().expect("macro values").get(name).copied()
    }

    pub fn enabled(&self, name: &str) -> bool {
        self.get(name).is_some_and(|value| value != 0)
    }

    pub fn callbacks(&self) -> ConfigCallbacks {
        ConfigCallbacks {
            cargo: bindgen::CargoCallbacks::new(),
            values: self.clone(),
        }
    }
}

/// Bindgen 0.71 keeps the first definition of a redefined numeric macro.
/// Const declarations make libclang evaluate its final expansion instead.
/// Application configurations use this path; default bindings retain their API.
#[derive(Debug)]
pub struct FinalMacros {
    pub names: Vec<String>,
    pub cargo: bindgen::CargoCallbacks,
}

impl FinalMacros {
    pub fn declarations(&self) -> String {
        // Bindgen evaluates integer constants through a signed i64, so a single
        // u64 probe cannot represent values above i64::MAX. Two u32 halves retain
        // every bit; the sign probe distinguishes a negative value from a large
        // unsigned value. These declarations are parsed by Clang, never linked.
        self.names
            .iter()
            .map(|name| {
                format!(
                    "#ifdef {name}\n\
             static const __UINT32_TYPE__ __lv_rs_final_low_{name} = (__UINT32_TYPE__)({name});\n\
             static const __UINT32_TYPE__ __lv_rs_final_high_{name} = (__UINT32_TYPE__)((__UINT64_TYPE__)({name}) >> 32);\n\
             static const __UINT32_TYPE__ __lv_rs_final_negative_{name} = (({name}) < 0);\n#endif\n"
                )
            })
            .collect()
    }

    pub fn project(bindings: &str) -> Result<String, String> {
        let mut probes = BTreeMap::<String, BTreeMap<String, u32>>::new();
        let mut result = String::new();
        let mut lines = bindings.lines();
        while let Some(line) = lines.next() {
            if let Some(probe) = line.strip_prefix("pub const __lv_rs_final_") {
                // Rustfmt can wrap long macro names/types before the value.
                let mut declaration = probe.to_owned();
                while !declaration.ends_with(';') {
                    declaration.push(' ');
                    declaration.push_str(lines.next().ok_or("unterminated macro probe")?.trim());
                }
                let (part, rest) = declaration.split_once('_').ok_or("invalid macro probe")?;
                let (name, rest) = rest.split_once(':').ok_or("invalid macro probe name")?;
                let (_, value) = rest.split_once('=').ok_or("missing macro value")?;
                let value = value
                    .trim_end_matches(';')
                    .trim()
                    .parse()
                    .map_err(|_| format!("unable to evaluate final macro {name}"))?;
                probes
                    .entry(name.to_owned())
                    .or_default()
                    .insert(part.to_owned(), value);
            } else if line.contains("__lv_rs_final_") {
                return Err("unable to evaluate a final LVGL integer macro".into());
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        for (name, parts) in probes {
            let get = |part| {
                parts
                    .get(part)
                    .copied()
                    .ok_or_else(|| format!("incomplete macro {name}"))
            };
            let bits = (u64::from(get("high")?) << 32) | u64::from(get("low")?);
            // Preserve bindgen's default macro types, independently of the C
            // expression's integer rank. Do not change ordinary u32 APIs to i32.
            let (kind, value) = if get("negative")? != 0 {
                let value = bits as i64;
                (
                    if i32::try_from(value).is_ok() {
                        "i32"
                    } else {
                        "i64"
                    },
                    value.to_string(),
                )
            } else {
                (
                    if u32::try_from(bits).is_ok() {
                        "u32"
                    } else {
                        "u64"
                    },
                    bits.to_string(),
                )
            };
            result.push_str(&format!("pub const {name}: {kind} = {value};\n"));
        }
        Ok(result)
    }
}

impl ParseCallbacks for FinalMacros {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if self.names.iter().any(|item| item == name) {
            MacroParsingBehavior::Ignore
        } else {
            MacroParsingBehavior::Default
        }
    }
    fn header_file(&self, name: &str) {
        if is_source_header(name) {
            self.cargo.header_file(name);
        }
    }
    fn include_file(&self, name: &str) {
        if is_source_header(name) {
            self.cargo.include_file(name);
        }
    }
    fn read_env_var(&self, name: &str) {
        self.cargo.read_env_var(name);
    }
}

#[derive(Debug)]
pub struct ConfigCallbacks {
    cargo: bindgen::CargoCallbacks,
    values: MacroValues,
}

impl ParseCallbacks for ConfigCallbacks {
    fn str_macro(&self, name: &str, _value: &[u8]) {
        // Some LV_USE_* names are paths (for example LV_USE_UEFI_INCLUDE),
        // not integer capabilities. Preserve their normal string bindings.
        self.values.0.lock().expect("macro values").remove(name);
    }

    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        // cexpr does not evaluate every C boolean expression. Record capability
        // names even when only libclang can evaluate their final value.
        if name.starts_with("LV_USE_")
            || name.starts_with("LV_FONT_MONTSERRAT_")
            || name.starts_with("LV_FONT_SOURCE_HAN_SANS_")
        {
            self.values
                .0
                .lock()
                .expect("macro values")
                .entry(name.into())
                .or_insert(0);
        }
        MacroParsingBehavior::Default
    }

    fn int_macro(&self, name: &str, value: i64) -> Option<IntKind> {
        if name.starts_with("LV_") {
            self.values
                .0
                .lock()
                .expect("macro values")
                .insert(name.into(), value);
        }
        None
    }

    fn header_file(&self, name: &str) {
        if is_source_header(name) {
            self.cargo.header_file(name);
        }
    }
    fn include_file(&self, name: &str) {
        if is_source_header(name) {
            self.cargo.include_file(name);
        }
    }
    fn read_env_var(&self, name: &str) {
        self.cargo.read_env_var(name);
    }
}
