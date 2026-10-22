// LVGL 9.6.0 bindgen + cc build (LVGL 9 line; originally ported from the
// mLupine-lvgl-9 branch which vendored 9.2.2).
//
// Vendored LVGL: vendor/lvgl-9.6/  (LVGL v9.6.0 release tarball
//   SHA256 b20ee3acc1bba13c62d854f9ebd62e4c51e0b443b1e0225892e86442defa84df
//   from https://github.com/lvgl/lvgl/archive/refs/tags/v9.6.0.tar.gz;
//   trimmed to the root build files + src/ + include/)
// Vendored config: vendor/include-9.6/lv_conf.h (derived from upstream
//   lv_conf_template.h with LV_USE_SNAPSHOT=1)
//
// This crate compiles the vendored provider. External precompiled LVGL
// integration requires a separate ABI/provider contract.

#[path = "build_support/inputs.rs"]
mod inputs;

use cc::Build;
use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

fn main() {
    let project_dir = canonicalize(PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()));
    let shims_dir = project_dir.join("shims");
    let vendor = project_dir.join("vendor");
    let lvgl_root = vendor.join("lvgl-9.6");
    let lvgl_src = lvgl_root.join("src");
    let lv_config_dir = vendor.join("include-9.6");
    let inputs = inputs::Inputs::from_env();

    if !lv_config_dir.join("lv_conf.h").exists() {
        panic!(
            "missing {} -- vendor LVGL 9.6 lv_conf.h not staged",
            lv_config_dir.join("lv_conf.h").display()
        );
    }

    println!(
        "cargo:rerun-if-changed={}",
        lv_config_dir.join("lv_conf.h").to_str().unwrap()
    );
    println!("cargo:rerun-if-changed={}", shims_dir.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", lvgl_src.to_str().unwrap());
    // Public headers: bindgen resolves lvgl.h -> include/lvgl/**, so edits
    // there must retrigger both cc and bindgen.
    println!(
        "cargo:rerun-if-changed={}",
        lvgl_root.join("lvgl.h").to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        lvgl_root.join("lvgl_private.h").to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        lvgl_root.join("include").to_str().unwrap()
    );
    println!("cargo:rerun-if-env-changed=LVGL_CFLAGS");
    println!("cargo:rerun-if-env-changed=LVGL_SYSROOT");

    // SDL2 desktop-simulator backend (LV_USE_SDL), enabled by the `sdl`
    // feature. The vendored lv_conf.h keeps the backend off so default builds
    // (e.g. ESP cross-compiles) never need SDL headers; the feature build
    // prepends a patched copy of the config via include-path order.
    let sdl_flags = if env::var("CARGO_FEATURE_SDL").is_ok() {
        let flags = discover_sdl2().unwrap_or_else(|| {
            panic!(
                "feature `sdl` requires SDL2: install it (e.g. `brew install sdl2` \
                 or `apt install libsdl2-dev`) so `pkg-config sdl2` or `sdl2-config` works"
            )
        });
        Some(flags)
    } else {
        None
    };

    // Widget and font selection (`minimal-config`, `widget-*`, `font-*`
    // features). Any change to the vendored config is written to a patched
    // copy that precedes the vendored directory on the include path.
    let base_conf =
        std::fs::read_to_string(lv_config_dir.join("lv_conf.h")).expect("read vendored lv_conf.h");
    let mut conf = base_conf.clone();
    if sdl_flags.is_some() && inputs.config.is_none() {
        conf = patch_lv_conf_for_sdl(&conf);
    }
    let demo_widgets = env::var("CARGO_FEATURE_LV_DEMO_WIDGETS").is_ok();
    let mut selection = ConfigSelection::from_env(|name| env::var(name).is_ok());
    if demo_widgets && selection.minimal && inputs.config.is_none() {
        println!(
            "cargo:warning=rs_lvgl_sys: `lv-demo-widgets` needs the full widget set; ignoring `minimal-config`"
        );
        selection.minimal = false;
    }
    let conf = if let Some(config) = &inputs.config {
        // Preprocess the original header in place so its quoted includes keep
        // their original meaning. Never patch an application's configuration.
        inputs::include_directive(config).unwrap()
    } else {
        patch_lv_conf_for_selection(&conf, &selection)
    };
    let patched_conf_dir = if conf != base_conf || inputs.config.is_some() {
        let dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("conf");
        std::fs::create_dir_all(&dir).expect("create conf dir");
        std::fs::write(dir.join("lv_conf.h"), &conf).expect("write conf/lv_conf.h");
        Some(dir)
    } else {
        None
    };

    let target = env::var("TARGET").expect("Cargo build scripts always have TARGET");
    let host = env::var("HOST").expect("Cargo build scripts always have HOST");
    let riscv = riscv_target_config(&target);
    let (triple, arch_args) = match riscv {
        Some((triple, args)) => (triple, args),
        None => (target.clone(), Vec::new()),
    };

    let mut cfg = Build::new();
    cfg.define("LV_CONF_INCLUDE_SIMPLE", Some("1"));
    if let Some(conf_dir) = &patched_conf_dir {
        // Must precede lv_config_dir: quoted includes search -I in order, so
        // `#include "lv_conf.h"` resolves to the patched copy, not the
        // vendored one.
        cfg.include(conf_dir);
    }
    cfg.include(&lvgl_root)
        .include(&lvgl_src)
        .include(&lv_config_dir)
        .include(&vendor)
        .warnings(false);

    // cc-rs hardcodes `-mabi=ilp32` for non-Linux RV32 targets, but Rust's
    // `riscv32imafc-unknown-none-elf` uses the single-float ABI. Pass the
    // target's real ISA/ABI after cc-rs's defaults so the C objects and the
    // Rust code agree; the same flags feed bindgen below.
    if target != host {
        for arg in &arch_args {
            cfg.flag(arg);
        }
    }

    if let Some(sdl) = &sdl_flags {
        for i in &sdl.include_dirs {
            cfg.include(i);
        }
        for (k, v) in &sdl.defines {
            cfg.define(k, Some(v.as_str()));
        }
    }

    let cflags_extra = env::var("LVGL_CFLAGS").unwrap_or_default();
    let cflags_extra: Vec<&str> = cflags_extra.split(',').filter(|s| !s.is_empty()).collect();
    for e in &cflags_extra {
        let mut it = e.split('=');
        cfg.define(it.next().unwrap(), it.next().unwrap_or_default());
    }

    let mut cc_args = vec![
        "-DLV_CONF_INCLUDE_SIMPLE=1".to_string(),
        format!("-I{}", lv_config_dir.display()),
        format!("-I{}", lvgl_root.display()),
        format!("-I{}", lvgl_src.display()),
        format!("-I{}", vendor.display()),
        "-fvisibility=default".to_string(),
    ];
    for e in &cflags_extra {
        cc_args.push(format!("-D{e}"));
    }

    // Bindgen needs the canonical clang triple and the target ISA/ABI: Rust
    // triples such as `riscv32imafc-unknown-none-elf` are not valid clang
    // driver triples. (`triple`/`arch_args` above carry the same values.)
    if target != host {
        cc_args.push("-target".to_string());
        cc_args.push(triple.clone());
        cc_args.extend(arch_args.iter().cloned());
    }

    // Bindgen invokes clang with `-target xtensa-esp32s3-espidf` and that
    // clang has no idea where the xtensa newlib headers live. Discover the
    // sysroot via xtensa-esp-elf-gcc on PATH and pass `-I <sysroot>/include`
    // so `<inttypes.h>` etc resolve. The same trick works for any
    // `xtensa-*-espidf` target. If the toolchain isn't on PATH (host build)
    // we silently skip -- there's nothing to add.
    if target.starts_with("xtensa-") {
        if let Some(sysroot_inc) = xtensa_sysroot_include() {
            cc_args.push(format!("-I{}", sysroot_inc));
        }
    }

    if let Some(conf_dir) = &patched_conf_dir {
        // Insert before lv_config_dir (cc_args[1]) so `#include "lv_conf.h"`
        // resolves to the patched copy, mirroring the cc include order.
        cc_args.insert(1, format!("-I{}", conf_dir.display()));
    }
    if let Some(sdl) = &sdl_flags {
        for i in &sdl.include_dirs {
            cc_args.push(format!("-I{}", i));
        }
        for (k, v) in &sdl.defines {
            cc_args.push(format!("-D{k}={v}"));
        }
    }

    if demo_widgets {
        cc_args.push(format!(
            "-I{}",
            lvgl_root.join("demos").join("widgets").display()
        ));
    }

    // bindgen runs against the host libclang, which has no idea where a
    // cross toolchain keeps newlib headers (inttypes.h, stdint.h, ...).
    // LVGL_SYSROOT points at the target sysroot (e.g. the riscv32-esp-elf
    // one printed by `riscv32-esp-elf-gcc -print-sysroot`).
    if let Ok(sysroot) = env::var("LVGL_SYSROOT") {
        if !sysroot.is_empty() {
            cc_args.push(format!(
                "-I{}",
                Path::new(&sysroot).join("include").display()
            ));
        }
    }

    for path in inputs.include_dirs() {
        cfg.include(path);
        cc_args.push(format!("-I{}", path.display()));
    }
    let macro_values = inputs::MacroValues::default();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut bindings_builder = bindgen::Builder::default()
        .header(shims_dir.join("rs_lvgl_sys.h").to_str().unwrap())
        .generate_comments(false)
        .derive_default(true)
        .layout_tests(false)
        .use_core()
        .ctypes_prefix("cty")
        .clang_args(cc_args.iter().map(|s| s.as_str()))
        .allowlist_type("lv_.*")
        .allowlist_type("_lv_.*")
        .allowlist_function("lv_.*")
        .allowlist_function("_lv_.*")
        .allowlist_var("LV_.*")
        // Built-in fonts are extern `lv_font_t` variables (lowercase), e.g.
        // lv_font_montserrat_14; expose the ones enabled in lv_conf.h.
        .allowlist_var("lv_font_.*")
        .blocklist_function("lv_log_add"); // varargs (va_list); not safely bindable on all targets
                                           // The public lvgl.h tree doesn't declare demo entry points; parse the
                                           // demo header too so lv_demo_widgets() lands in the bindings.
    if demo_widgets {
        bindings_builder = bindings_builder.header(
            lvgl_root
                .join("demos/widgets/lv_demo_widgets.h")
                .to_str()
                .unwrap(),
        );
    }
    for header in &inputs.headers {
        bindings_builder = bindings_builder
            .header(header.to_str().expect("validated header path"))
            .allowlist_file(inputs::file_pattern(header));
    }
    let mut bindings = bindings_builder
        .clone()
        .parse_callbacks(Box::new(macro_values.callbacks()))
        .generate()
        .expect("Unable to generate LVGL 9.6 bindings")
        .to_string();

    if inputs.config.is_some() {
        let callbacks = inputs::FinalMacros {
            names: macro_values.names(),
            cargo: bindgen::CargoCallbacks::new(),
        };
        let final_macros = out_path.join("lvgl-final-macros.h");
        std::fs::write(&final_macros, callbacks.declarations())
            .expect("write final macro declarations");
        bindings = bindings_builder
            .header(final_macros.to_str().expect("UTF-8 OUT_DIR"))
            .allowlist_var("__lv_rs_final_.*")
            .parse_callbacks(Box::new(callbacks))
            .generate()
            .expect("Unable to evaluate final LVGL configuration macros")
            .to_string();
        bindings = inputs::FinalMacros::project(&bindings)
            .expect("Unable to project final LVGL integer macros");
        macro_values.read_constants(&bindings);
        validate_application_config(&macro_values, &selection, sdl_flags.is_some(), demo_widgets)
            .unwrap_or_else(|error| panic!("{error}"));
    }
    // Metadata comes from the target preprocessor, including expressions and
    // transitive headers, rather than a textual approximation of the config.
    println!(
        "cargo:widgets={}",
        OPTIONAL_WIDGETS
            .iter()
            .filter(|w| macro_values.enabled(&format!("LV_USE_{}", w.conf)))
            .map(|w| w.feature)
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "cargo:optional_widgets={}",
        OPTIONAL_WIDGETS
            .iter()
            .map(|w| w.feature)
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut sizes = FONT_SIZES.to_vec();
    sizes.push(14);
    sizes.sort_unstable();
    println!(
        "cargo:fonts={}",
        sizes
            .iter()
            .filter(|size| macro_values.enabled(&format!("LV_FONT_MONTSERRAT_{size}")))
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );

    // C and C++ share the exact include order, defines and target ABI flags.
    // Only the selected vendored ThorVG provider requires a C++ toolchain.
    let mut cpp = cfg.clone();
    add_c_files(&mut cfg, &lvgl_src);
    add_c_files(&mut cfg, &shims_dir);
    if let Some(path) = &inputs.extra_c {
        add_c_files(&mut cfg, path);
    }

    // Official widgets demo (`demo-widgets` cargo feature): compile the
    // vendored demo sources. LV_USE_DEMO_WIDGETS itself is enabled in the
    // vendored lv_conf.h.
    if demo_widgets {
        let widgets_dir = lvgl_root.join("demos").join("widgets");
        add_c_files(&mut cfg, &widgets_dir);
        // lv_demo_widgets refers to the shared demo-args helper, which is
        // defined at demos/lv_demos.c (other demos' calls compile out).
        cfg.file(lvgl_root.join("demos").join("lv_demos.c"));
    }

    cfg.compile("lvgl");
    if macro_values.enabled("LV_USE_THORVG") {
        cpp.cpp(true).std("c++14");
        // Upstream ignores its Meson-generated config.h. Supply a canonical
        // configuration inside OUT_DIR so unpacked packages build too. Force
        // it first to make an ignored local Meson file unable to override it.
        let thorvg_conf = PathBuf::from(env::var("OUT_DIR").unwrap()).join("thorvg-conf");
        std::fs::create_dir_all(&thorvg_conf).expect("create ThorVG config dir");
        let header = thorvg_conf.join("config.h");
        std::fs::write(&header, include_str!("shims/thorvg_config.h"))
            .expect("write ThorVG config");
        cpp.include(&thorvg_conf);
        if cpp.get_compiler().is_like_msvc() {
            cpp.flag(format!("/FI{}", header.display()));
        } else {
            cpp.flag("-include")
                .flag(header.to_str().expect("UTF-8 OUT_DIR"));
        }
        add_source_files(&mut cpp, lvgl_src.join("libs/thorvg"), "cpp");
        cpp.compile("lvgl_thorvg");
    }

    if let Some(sdl) = &sdl_flags {
        for dir in &sdl.link_dirs {
            println!("cargo:rustc-link-search=native={dir}");
        }
        for lib in &sdl.libs {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
        for fw in &sdl.frameworks {
            println!("cargo:rustc-link-lib=framework={fw}");
        }
    }

    std::fs::write(out_path.join("bindings.rs"), bindings).expect("Can't write bindings!");
    // Dependents that generate code from the bindings must read this file:
    // a build-dependency on rs_lvgl_sys is compiled with its own features and
    // may see a different widget set.
    println!("cargo:bindings={}", out_path.join("bindings.rs").display());
}

fn validate_application_config(
    values: &inputs::MacroValues,
    selection: &ConfigSelection,
    sdl: bool,
    demo: bool,
) -> Result<(), String> {
    let require = |name: &str| {
        if values.enabled(name) {
            Ok(())
        } else {
            Err(format!("LVGL_CONFIG_PATH conflicts with a requested Cargo capability: {name} must be enabled"))
        }
    };
    for widget in OPTIONAL_WIDGETS {
        if selection.widgets.contains(widget.feature) {
            require(&format!("LV_USE_{}", widget.conf))?;
        }
    }
    for size in &selection.fonts {
        require(&format!("LV_FONT_MONTSERRAT_{size}"))?;
    }
    for (feature, name) in OPTIONAL_FLAGS {
        if selection.flags.contains(feature) {
            require(name)?;
        }
    }
    if sdl {
        require("LV_USE_SDL")?;
    }
    if values.enabled("LV_USE_SDL") && !sdl {
        return Err("LVGL_CONFIG_PATH enables LV_USE_SDL; also enable the sdl Cargo feature for native linking".into());
    }
    if demo {
        require("LV_USE_DEMO_WIDGETS")?;
    }
    if values.enabled("LV_USE_THORVG") && !values.enabled("LV_USE_THORVG_INTERNAL") {
        return Err(
            "LVGL_CONFIG_PATH must use LV_USE_THORVG_INTERNAL with the vendored provider".into(),
        );
    }
    Ok(())
}

/// An LVGL widget that the `minimal-config` feature can leave out.
struct OptionalWidget {
    /// Cargo feature suffix: `widget-<feature>`.
    feature: &'static str,
    /// `LV_USE_<conf>` in lv_conf.h.
    conf: &'static str,
    /// Widgets this one cannot be compiled without.
    requires: &'static [&'static str],
}

const fn widget(
    feature: &'static str,
    conf: &'static str,
    requires: &'static [&'static str],
) -> OptionalWidget {
    OptionalWidget {
        feature,
        conf,
        requires,
    }
}

/// Every widget of the vendored config except label and button, which stay
/// enabled because most widgets and themes build on them. Dependencies follow
/// LVGL's own `#error` checks and unguarded calls into other widgets; the
/// `widget_requires_match_lvgl_sources` test checks them against the vendored
/// sources.
const OPTIONAL_WIDGETS: &[OptionalWidget] = &[
    widget("animimage", "ANIMIMG", &["image"]),
    widget("arc", "ARC", &[]),
    widget("arclabel", "ARCLABEL", &[]),
    widget("bar", "BAR", &[]),
    widget("barcode", "BARCODE", &["canvas", "image"]),
    widget("buttonmatrix", "BUTTONMATRIX", &[]),
    widget("calendar", "CALENDAR", &["buttonmatrix"]),
    widget("canvas", "CANVAS", &["image"]),
    widget("chart", "CHART", &[]),
    widget("checkbox", "CHECKBOX", &[]),
    widget("dropdown", "DROPDOWN", &[]),
    widget("gif", "GIF", &["image"]),
    widget("image", "IMAGE", &[]),
    widget(
        "ime-pinyin",
        "IME_PINYIN",
        &["keyboard", "buttonmatrix", "textarea"],
    ),
    widget("imagebutton", "IMAGEBUTTON", &[]),
    widget("keyboard", "KEYBOARD", &["buttonmatrix", "textarea"]),
    widget("led", "LED", &[]),
    widget("line", "LINE", &[]),
    widget("list", "LIST", &["image"]),
    widget("lottie", "LOTTIE", &["canvas", "image"]),
    widget("menu", "MENU", &["image"]),
    widget("msgbox", "MSGBOX", &["image"]),
    widget("qrcode", "QRCODE", &["canvas"]),
    widget("roller", "ROLLER", &[]),
    widget("scale", "SCALE", &["image", "line"]),
    widget("slider", "SLIDER", &["bar"]),
    widget("span", "SPAN", &[]),
    widget("spinbox", "SPINBOX", &["textarea"]),
    widget("spinner", "SPINNER", &["arc"]),
    widget("switch", "SWITCH", &[]),
    widget("table", "TABLE", &[]),
    widget("tabview", "TABVIEW", &[]),
    widget("textarea", "TEXTAREA", &[]),
    widget("tileview", "TILEVIEW", &[]),
    widget("win", "WIN", &["image"]),
];

/// Built-in Montserrat sizes selectable with `font-montserrat-<size>`.
/// Montserrat 14 is LV_FONT_DEFAULT and always stays enabled.
const FONT_SIZES: &[u32] = &[
    8, 10, 12, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48,
];

const OPTIONAL_FLAGS: &[(&str, &str)] = &[
    ("font-cjk-14", "LV_FONT_SOURCE_HAN_SANS_SC_14_CJK"),
    ("font-cjk-16", "LV_FONT_SOURCE_HAN_SANS_SC_16_CJK"),
    ("gridnav", "LV_USE_GRIDNAV"),
];

/// Requested widgets and fonts.
///
/// Without `minimal`, the vendored settings are kept and the features only
/// add to them. With `minimal`, every optional widget and font is off unless
/// its feature (or a widget that requires it) is enabled.
#[derive(Default)]
struct ConfigSelection {
    minimal: bool,
    widgets: BTreeSet<&'static str>,
    fonts: BTreeSet<u32>,
    flags: BTreeSet<&'static str>,
}

impl ConfigSelection {
    fn from_env(is_set: impl Fn(&str) -> bool) -> Self {
        let feature = |name: &str| {
            is_set(&format!(
                "CARGO_FEATURE_{}",
                name.to_uppercase().replace('-', "_")
            ))
        };
        Self {
            minimal: feature("minimal-config"),
            flags: OPTIONAL_FLAGS
                .iter()
                .filter(|(name, _)| feature(name))
                .map(|(name, _)| *name)
                .collect(),
            widgets: OPTIONAL_WIDGETS
                .iter()
                .filter(|w| feature(&format!("widget-{}", w.feature)))
                .map(|w| w.feature)
                .collect(),
            fonts: FONT_SIZES
                .iter()
                .copied()
                .filter(|size| feature(&format!("font-montserrat-{size}")))
                .collect(),
        }
    }
}

/// `Some(value)` when `line` is exactly `#define <name> <0|1>`.
fn define_value(line: &str, name: &str) -> Option<bool> {
    let rest = line.strip_prefix("#define ")?.strip_prefix(name)?;
    match rest.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn conf_value(conf: &str, name: &str) -> Option<bool> {
    conf.lines().find_map(|line| define_value(line, name))
}

fn patch_lv_conf_for_selection(base: &str, selection: &ConfigSelection) -> String {
    let widget_name = |w: &OptionalWidget| format!("LV_USE_{}", w.conf);
    let mut enabled = BTreeSet::new();
    for w in OPTIONAL_WIDGETS {
        let current = conf_value(base, &widget_name(w)).unwrap_or_else(|| {
            panic!(
                "lv_conf.h has no `#define {} 0|1`; update OPTIONAL_WIDGETS",
                widget_name(w)
            )
        });
        if selection.widgets.contains(w.feature) || (current && !selection.minimal) {
            enabled.insert(w.feature);
        }
    }
    // Close over dependencies; the list is shallow, so a few passes suffice.
    loop {
        let before = enabled.len();
        for w in OPTIONAL_WIDGETS {
            if enabled.contains(w.feature) {
                enabled.extend(w.requires.iter().copied());
            }
        }
        if enabled.len() == before {
            break;
        }
    }
    let font_name = |size: u32| format!("LV_FONT_MONTSERRAT_{size}");
    let mut fonts = BTreeSet::new();
    for &size in FONT_SIZES {
        let current = conf_value(base, &font_name(size)).unwrap_or_else(|| {
            panic!(
                "lv_conf.h has no `#define {} 0|1`; update FONT_SIZES",
                font_name(size)
            )
        });
        if selection.fonts.contains(&size) || (current && !selection.minimal) {
            fonts.insert(size);
        }
    }

    let mut out = String::with_capacity(base.len());
    for line in base.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let ending = &line[body.len()..];
        let widget = OPTIONAL_WIDGETS
            .iter()
            .find(|w| define_value(body, &widget_name(w)).is_some());
        let font = FONT_SIZES
            .iter()
            .copied()
            .find(|&size| define_value(body, &font_name(size)).is_some());
        let flag = OPTIONAL_FLAGS
            .iter()
            .find(|(_, name)| define_value(body, name).is_some());
        let replacement = if let Some((feature, name)) = flag {
            Some((
                (*name).to_string(),
                selection.flags.contains(feature)
                    || (!selection.minimal && conf_value(base, name) == Some(true)),
            ))
        } else if enabled.contains("lottie")
            && [
                "LV_USE_THORVG",
                "LV_USE_VECTOR_GRAPHIC",
                "LV_USE_MATRIX",
                "LV_USE_FLOAT",
            ]
            .iter()
            .any(|name| define_value(body, name).is_some())
        {
            let name = [
                "LV_USE_THORVG",
                "LV_USE_VECTOR_GRAPHIC",
                "LV_USE_MATRIX",
                "LV_USE_FLOAT",
            ]
            .into_iter()
            .find(|name| define_value(body, name).is_some())
            .unwrap();
            Some((name.to_string(), true))
        } else if let Some(w) = widget {
            Some((widget_name(w), enabled.contains(w.feature)))
        } else if let Some(size) = font {
            Some((font_name(size), fonts.contains(&size)))
        } else if define_value(body, "LV_USE_CALENDAR_HEADER_DROPDOWN") == Some(true) {
            // The calendar header dropdown is an lv_dropdown.
            Some((
                "LV_USE_CALENDAR_HEADER_DROPDOWN".to_string(),
                enabled.contains("dropdown"),
            ))
        } else {
            None
        };
        match replacement {
            Some((name, on)) => {
                out.push_str(&format!("#define {name} {}", u8::from(on)));
                out.push_str(ending);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// Optional widgets (feature suffixes) enabled by `conf`.
#[cfg(test)]
fn enabled_widgets(conf: &str) -> Vec<&'static str> {
    OPTIONAL_WIDGETS
        .iter()
        .filter(|w| conf_value(conf, &format!("LV_USE_{}", w.conf)) == Some(true))
        .map(|w| w.feature)
        .collect()
}

/// Montserrat sizes enabled by `conf`, including the default 14.
#[cfg(test)]
fn enabled_fonts(conf: &str) -> Vec<u32> {
    let mut sizes: Vec<u32> = FONT_SIZES
        .iter()
        .copied()
        .filter(|&size| conf_value(conf, &format!("LV_FONT_MONTSERRAT_{size}")) == Some(true))
        .collect();
    sizes.push(14);
    sizes.sort_unstable();
    sizes
}

/// Map a Rust `riscv32*` target triple to the canonical clang triple and the
/// matching `-march`/`-mabi` pair.
///
/// The ABI must follow the Rust target: `riscv32imafc` uses `ilp32f`, and a
/// soft-float ABI for the C objects would break linkage and floating-point
/// calling conventions. cc-rs does not derive this for bare-metal RV32 and
/// hardcodes `-mabi=ilp32`, so the build passes these flags explicitly.
fn riscv_target_config(target: &str) -> Option<(String, Vec<String>)> {
    // Only bare-metal targets need this override. Linux RV32 targets have
    // their own ABI selection in cc-rs (`rv32gc`/`ilp32d`) and must keep it.
    if !target.contains("-none-") {
        return None;
    }
    let isa = target
        .strip_prefix("riscv32")?
        .split('-')
        .next()
        .unwrap_or_default();
    if isa.is_empty() {
        return None;
    }
    let mabi = if isa.contains('d') {
        "ilp32d"
    } else if isa.contains('f') {
        "ilp32f"
    } else {
        "ilp32"
    };
    Some((
        "riscv32-unknown-elf".to_string(),
        vec![format!("-march=rv32{isa}"), format!("-mabi={mabi}")],
    ))
}

fn add_c_files(build: &mut cc::Build, path: impl AsRef<Path>) {
    add_source_files(build, path, "c");
}

fn add_source_files(build: &mut cc::Build, path: impl AsRef<Path>, extension: &str) {
    for e in path.as_ref().read_dir().expect("read_dir failed") {
        let e = e.unwrap();
        let p = e.path();
        if e.file_type().unwrap().is_dir() {
            add_source_files(build, &p, extension);
        } else if p.extension().and_then(|s| s.to_str()) == Some(extension) {
            build.file(&p);
        }
    }
}

/// Compile/link flags for SDL2, discovered via `pkg-config sdl2` with a
/// `sdl2-config` fallback (Homebrew ships both).
struct SdlFlags {
    include_dirs: Vec<String>,
    defines: Vec<(String, String)>,
    link_dirs: Vec<String>,
    libs: Vec<String>,
    frameworks: Vec<String>,
}

impl SdlFlags {
    fn empty() -> Self {
        SdlFlags {
            include_dirs: Vec::new(),
            defines: Vec::new(),
            link_dirs: Vec::new(),
            libs: Vec::new(),
            frameworks: Vec::new(),
        }
    }
}

fn discover_sdl2() -> Option<SdlFlags> {
    use std::process::Command;

    fn probe(tool: &str, arg: &str) -> Option<String> {
        let out = Command::new(tool).arg(arg).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn absorb(out: &str, flags: &mut SdlFlags) {
        let mut fw_next = false;
        for tok in out.split_whitespace() {
            if fw_next {
                flags.frameworks.push(tok.to_string());
                fw_next = false;
            } else if let Some(dir) = tok.strip_prefix("-I") {
                if dir.is_empty() {
                    continue;
                }
                flags.include_dirs.push(dir.to_string());
                // `sdl2-config` only exposes the .../include/SDL2 leaf, while
                // LVGL includes "SDL2/SDL.h" relative to the include root.
                if Path::new(dir).file_name().and_then(|s| s.to_str()) == Some("SDL2") {
                    if let Some(parent) = Path::new(dir).parent() {
                        let parent = parent.to_string_lossy().to_string();
                        if !flags.include_dirs.contains(&parent) {
                            flags.include_dirs.push(parent);
                        }
                    }
                }
            } else if let Some(def) = tok.strip_prefix("-D") {
                let (k, v) = def.split_once('=').unwrap_or((def, ""));
                // A bare `-Dfoo` means "defined as 1" for C compilers; keep
                // the cc and bindgen args identical by normalizing here.
                let v = if v.is_empty() { "1" } else { v };
                flags.defines.push((k.to_string(), v.to_string()));
            } else if let Some(dir) = tok.strip_prefix("-L") {
                if !dir.is_empty() {
                    flags.link_dirs.push(dir.to_string());
                }
            } else if let Some(lib) = tok.strip_prefix("-l") {
                if !lib.is_empty() {
                    flags.libs.push(lib.to_string());
                }
            } else if tok == "-framework" {
                fw_next = true;
            } else if let Some(rest) = tok.strip_prefix("-Wl,") {
                // e.g. `-Wl,-framework,Cocoa` from pkg-config --libs
                let mut parts = rest.split(',');
                while let Some(part) = parts.next() {
                    if part == "-framework" {
                        if let Some(fw) = parts.next() {
                            flags.frameworks.push(fw.to_string());
                        }
                    }
                    // other linker flags (rpath, ...) are left to the dylib
                }
            } else if tok.starts_with('-') {
                println!("cargo:warning=rs_lvgl_sys: ignoring unrecognized SDL2 flag `{tok}`");
            }
        }
    }

    // pkg-config is preferred, but both queries must succeed: a partial
    // result (e.g. a broken .pc) would silently drop include or link flags.
    let mut flags = SdlFlags::empty();
    let mut pkg_errs: Vec<String> = Vec::new();
    let mut pkg_ok = true;
    for arg in ["--cflags", "--libs"] {
        match probe("pkg-config", arg) {
            Some(out) => absorb(&out, &mut flags),
            None => {
                pkg_ok = false;
                pkg_errs.push(format!("pkg-config {arg} failed"));
            }
        }
    }
    if !pkg_ok {
        flags = SdlFlags::empty();
        let cfg_ok = probe("sdl2-config", "--cflags").is_some_and(|out| {
            absorb(&out, &mut flags);
            true
        });
        let libs_ok = probe("sdl2-config", "--libs").is_some_and(|out| {
            absorb(&out, &mut flags);
            true
        });
        if !(cfg_ok && libs_ok) {
            for e in &pkg_errs {
                println!("cargo:warning=rs_lvgl_sys: {e}");
            }
            println!("cargo:warning=rs_lvgl_sys: sdl2-config fallback failed; is SDL2 installed?");
            return None;
        }
    }
    Some(flags)
}

/// Patch the vendored lv_conf.h for SDL simulator builds: SDL backend on,
/// C allocator (the 64 kB embedded TLSF pool cannot host desktop-size
/// snapshot buffers), warnings to stdout. Textual patching against the
/// pinned vendored file keeps default builds untouched. Accept settings that
/// already have the required value (in particular, a CLIB base allocator).
fn patch_lv_conf_for_sdl(base: &str) -> String {
    let edits = [
        ("#define LV_USE_SDL 0", "#define LV_USE_SDL 1"),
        (
            "#define LV_USE_STDLIB_MALLOC LV_STDLIB_BUILTIN",
            "#define LV_USE_STDLIB_MALLOC LV_STDLIB_CLIB",
        ),
        ("#define LV_USE_LOG 0", "#define LV_USE_LOG 1"),
        ("#define LV_LOG_PRINTF 0", "#define LV_LOG_PRINTF 1"),
    ];
    let mut s = base.to_string();
    for (from, to) in edits {
        assert!(
            s.contains(from) || s.contains(to),
            "lv_conf.h contains neither `{from}` nor `{to}`; update patch_lv_conf_for_sdl"
        );
        s = s.replace(from, to);
    }
    s
}

/// Locate the xtensa-esp-elf newlib include dir by asking
/// `xtensa-esp-elf-gcc -print-sysroot` (then appending `/include`).
/// Returns `None` if the tool is missing or fails.
fn xtensa_sysroot_include() -> Option<String> {
    use std::process::Command;
    let out = Command::new("xtensa-esp-elf-gcc")
        .arg("-print-sysroot")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sysroot.is_empty() {
        return None;
    }
    Some(format!("{sysroot}/include"))
}

fn canonicalize(path: impl AsRef<Path>) -> PathBuf {
    let canonicalized = path.as_ref().canonicalize().unwrap();
    let s = canonicalized.to_string_lossy().to_string();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(s.as_str()))
}
