//! Regression tests for SDL configuration, widget and font selection, and the
//! widgets demo package inputs.

// Build scripts are not Cargo test targets. Include the actual implementation
// here so these tests cannot silently diverge from the build-time patch logic.
#[allow(dead_code)]
mod build_script {
    include!("../build.rs");

    #[test]
    fn application_capabilities_follow_preprocessed_expressions() {
        let source = "#define CHOSEN 1\n#define LV_USE_SLIDER (CHOSEN && 1)\n#define LV_FONT_MONTSERRAT_20 (10 * 2 == 20)\n#define LV_DPI_DEF 130\n#undef LV_DPI_DEF\n#define LV_DPI_DEF (170 + 3)\n";
        let values = inputs::MacroValues::default();
        bindgen::Builder::default()
            .header_contents("application-config.h", source)
            .parse_callbacks(Box::new(values.callbacks()))
            .generate()
            .unwrap();
        let callbacks = inputs::FinalMacros {
            names: values.names(),
            cargo: bindgen::CargoCallbacks::new(),
        };
        let bindings = bindgen::Builder::default()
            .header_contents("final.h", &format!("{source}{}", callbacks.declarations()))
            .parse_callbacks(Box::new(callbacks))
            .generate()
            .unwrap();
        let bindings = inputs::FinalMacros::project(&bindings.to_string()).unwrap();
        values.read_constants(&bindings);
        assert!(bindings.contains("LV_DPI_DEF"));
        assert_eq!(values.get("LV_DPI_DEF"), Some(173));
        assert!(validate_application_config(
            &values,
            &selection(true, &["slider"], &[20]),
            false,
            false
        )
        .is_ok());
        assert!(validate_application_config(
            &values,
            &selection(false, &["chart"], &[]),
            false,
            false
        )
        .unwrap_err()
        .contains("LV_USE_CHART"));
    }

    #[test]
    fn application_paths_reject_unrepresentable_inputs() {
        assert!(inputs::checked_path(
            "LVGL_CONFIG_PATH",
            PathBuf::from("relative/lv_conf.h"),
            false
        )
        .unwrap_err()
        .contains("absolute path"));
        assert!(inputs::include_directive(Path::new("/config/invalid\nheader.h")).is_err());
        assert_eq!(
            inputs::include_directive(Path::new("/config/with spaces/lv_conf.h")).unwrap(),
            "#include \"/config/with spaces/lv_conf.h\"\n"
        );
    }

    #[test]
    fn final_macro_values_preserve_integer_consumer_types_without_truncation() {
        let source = "\
#define LV_POS 7
#undef LV_POS
#define LV_POS (170 + 3)
#define LV_NEG (-42)
#define LV_MIN32 (-2147483647 - 1)
#define LV_WIDE_NEG (-2147483649LL)
#define LV_MAX32 4294967295U
#define LV_WIDE 4294967296ULL
#define LV_MAX64 18446744073709551615ULL
#define LV_USE_EXPR (1 && 2)
#define LV_USE_SOFTWARE_DRAWING_THIS_NAME_IS_LONG_ENOUGH_TO_WRAP_WITH_BINDGEN_CTYPES_PREFIX 1
#define LV_USE_UEFI_INCLUDE \"myefi.h\"
";
        let values = inputs::MacroValues::default();
        bindgen::Builder::default()
            .header_contents("macros.h", source)
            .parse_callbacks(Box::new(values.callbacks()))
            .generate()
            .unwrap();
        let callbacks = inputs::FinalMacros {
            names: values.names(),
            cargo: bindgen::CargoCallbacks::new(),
        };
        let bindings = bindgen::Builder::default()
            .header_contents("final.h", &format!("{source}{}", callbacks.declarations()))
            .parse_callbacks(Box::new(callbacks))
            .generate()
            .unwrap();
        // Compile a real Rust consumer: equal numeric values with an incompatible
        // integer type must fail, as must truncating the wider configuration.
        let bindings = inputs::FinalMacros::project(&bindings.to_string()).unwrap();
        let directory = env::temp_dir().join(format!("lvgl-macro-types-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let consumer = directory.join("consumer.rs");
        std::fs::write(
            &consumer,
            format!(
                "{bindings}\n{}",
                r#"
const _: u32 = LV_POS;
const _: i32 = LV_NEG;
const _: i32 = LV_MIN32;
const _: i64 = LV_WIDE_NEG;
const _: u32 = LV_MAX32;
const _: u64 = LV_WIDE;
const _: u64 = LV_MAX64;
const _: u32 = LV_USE_EXPR;
const _: () = assert!(LV_POS == 173 && LV_NEG == -42 && LV_MIN32 == i32::MIN);
const _: () = assert!(LV_WIDE_NEG == -2147483649 && LV_MAX32 == u32::MAX);
const _: () = assert!(LV_WIDE == 4294967296 && LV_MAX64 == u64::MAX && LV_USE_EXPR == 1);
"#
            ),
        )
        .unwrap();
        let result = std::process::Command::new(env::var_os("RUSTC").unwrap_or("rustc".into()))
            .args(["--crate-type=lib", "--emit=metadata", "--edition=2021"])
            .arg(&consumer)
            .arg("-o")
            .arg(directory.join("consumer.rmeta"))
            .output()
            .unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    const BASE_CONF: &str = "\
#define LV_USE_SDL 0
#define LV_USE_STDLIB_MALLOC LV_STDLIB_BUILTIN
#define LV_USE_LOG 0
#define LV_LOG_PRINTF 0
#define LV_FONT_MONTSERRAT_28 1
";

    const SDL_CONF: &str = "\
#define LV_USE_SDL 1
#define LV_USE_STDLIB_MALLOC LV_STDLIB_CLIB
#define LV_USE_LOG 1
#define LV_LOG_PRINTF 1
#define LV_FONT_MONTSERRAT_28 1
";

    #[test]
    fn sdl_patch_accepts_builtin_allocator() {
        assert_eq!(patch_lv_conf_for_sdl(BASE_CONF), SDL_CONF);
    }

    #[test]
    fn sdl_patch_accepts_clib_allocator() {
        let base = BASE_CONF.replace("LV_STDLIB_BUILTIN", "LV_STDLIB_CLIB");
        assert_eq!(patch_lv_conf_for_sdl(&base), SDL_CONF);
    }

    #[test]
    fn sdl_patch_is_idempotent() {
        assert_eq!(patch_lv_conf_for_sdl(SDL_CONF), SDL_CONF);
    }

    #[test]
    fn sdl_patch_preserves_other_vendored_settings() {
        let base = include_str!("../vendor/include-9.6/lv_conf.h");
        let expected = base
            .replace("#define LV_USE_SDL 0", "#define LV_USE_SDL 1")
            .replace(
                "#define LV_USE_STDLIB_MALLOC LV_STDLIB_BUILTIN",
                "#define LV_USE_STDLIB_MALLOC LV_STDLIB_CLIB",
            )
            .replace("#define LV_USE_LOG 0", "#define LV_USE_LOG 1")
            .replace("#define LV_LOG_PRINTF 0", "#define LV_LOG_PRINTF 1");
        assert_eq!(patch_lv_conf_for_sdl(base), expected);
    }

    #[test]
    #[should_panic(expected = "LV_USE_STDLIB_MALLOC")]
    fn sdl_patch_rejects_unsupported_allocator() {
        let base = BASE_CONF.replace("LV_STDLIB_BUILTIN", "LV_STDLIB_CUSTOM");
        patch_lv_conf_for_sdl(&base);
    }

    #[test]
    #[should_panic(expected = "LV_USE_STDLIB_MALLOC")]
    fn sdl_patch_rejects_missing_allocator() {
        let base = BASE_CONF.replace("#define LV_USE_STDLIB_MALLOC LV_STDLIB_BUILTIN\n", "");
        patch_lv_conf_for_sdl(&base);
    }

    fn vendored_conf() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/include-9.6/lv_conf.h"
        ))
        .unwrap()
    }

    fn selection(minimal: bool, widgets: &[&'static str], fonts: &[u32]) -> ConfigSelection {
        ConfigSelection {
            minimal,
            widgets: widgets.iter().copied().collect(),
            fonts: fonts.iter().copied().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn selection_without_features_keeps_the_vendored_conf() {
        let base = vendored_conf();
        assert_eq!(
            patch_lv_conf_for_selection(&base, &selection(false, &[], &[])),
            base
        );
    }

    #[test]
    fn minimal_selection_keeps_only_requested_widgets_and_dependencies() {
        let conf = patch_lv_conf_for_selection(
            &vendored_conf(),
            &selection(true, &["slider", "keyboard"], &[28]),
        );
        assert_eq!(
            enabled_widgets(&conf),
            ["bar", "buttonmatrix", "keyboard", "slider", "textarea"]
        );
        assert_eq!(enabled_fonts(&conf), [14, 28]);
        assert_eq!(conf_value(&conf, "LV_USE_LABEL"), Some(true));
        assert_eq!(conf_value(&conf, "LV_USE_BUTTON"), Some(true));
        assert_eq!(conf_value(&conf, "LV_FONT_MONTSERRAT_14"), Some(true));
    }

    #[test]
    fn calendar_header_dropdown_follows_dropdown() {
        let without =
            patch_lv_conf_for_selection(&vendored_conf(), &selection(true, &["calendar"], &[]));
        assert_eq!(
            conf_value(&without, "LV_USE_CALENDAR_HEADER_DROPDOWN"),
            Some(false)
        );
        assert_eq!(enabled_widgets(&without), ["buttonmatrix", "calendar"]);
        let with = patch_lv_conf_for_selection(
            &vendored_conf(),
            &selection(true, &["calendar", "dropdown"], &[]),
        );
        assert_eq!(
            conf_value(&with, "LV_USE_CALENDAR_HEADER_DROPDOWN"),
            Some(true)
        );
    }

    #[test]
    fn features_add_to_the_vendored_conf() {
        let conf = patch_lv_conf_for_selection(&vendored_conf(), &selection(false, &[], &[20]));
        assert_eq!(enabled_fonts(&conf), [12, 14, 16, 20, 28, 48]);
        assert_eq!(enabled_widgets(&conf), enabled_widgets(&vendored_conf()));
    }

    #[test]
    fn selection_reads_cargo_features() {
        let set = [
            "CARGO_FEATURE_MINIMAL_CONFIG",
            "CARGO_FEATURE_WIDGET_CHART",
            "CARGO_FEATURE_FONT_MONTSERRAT_48",
        ];
        let selection = ConfigSelection::from_env(|name| set.contains(&name));
        assert!(selection.minimal);
        assert_eq!(
            selection.widgets.iter().copied().collect::<Vec<_>>(),
            ["chart"]
        );
        assert_eq!(selection.fonts.iter().copied().collect::<Vec<_>>(), [48]);
    }

    #[test]
    fn optional_extension_dependencies_and_fonts_are_additive() {
        for (name, required) in [
            ("qrcode", vec!["canvas", "image", "qrcode"]),
            ("barcode", vec!["barcode", "canvas", "image"]),
            ("gif", vec!["gif", "image"]),
            ("lottie", vec!["canvas", "image", "lottie"]),
            (
                "ime-pinyin",
                vec!["buttonmatrix", "ime-pinyin", "keyboard", "textarea"],
            ),
        ] {
            let conf =
                patch_lv_conf_for_selection(&vendored_conf(), &selection(true, &[name], &[]));
            assert_eq!(enabled_widgets(&conf), required, "{name}");
            assert_eq!(conf_value(&conf, "LV_USE_THORVG"), Some(name == "lottie"));
        }
        let selected = ConfigSelection::from_env(|name| {
            [
                "CARGO_FEATURE_MINIMAL_CONFIG",
                "CARGO_FEATURE_FONT_CJK_16",
                "CARGO_FEATURE_GRIDNAV",
            ]
            .contains(&name)
        });
        let conf = patch_lv_conf_for_selection(&vendored_conf(), &selected);
        assert_eq!(
            conf_value(&conf, "LV_FONT_SOURCE_HAN_SANS_SC_16_CJK"),
            Some(true)
        );
        assert_eq!(
            conf_value(&conf, "LV_FONT_SOURCE_HAN_SANS_SC_14_CJK"),
            Some(false)
        );
        assert_eq!(conf_value(&conf, "LV_USE_GRIDNAV"), Some(true));
        assert_eq!(enabled_fonts(&conf), [14]);
    }

    #[test]
    fn riscv32_imafc_uses_single_float_abi() {
        // Rust's riscv32imafc target is ilp32f; CC objects must match so the
        // linker and floating-point calling convention agree.
        let (triple, args) = riscv_target_config("riscv32imafc-unknown-none-elf").unwrap();
        assert_eq!(triple, "riscv32-unknown-elf");
        assert_eq!(args, ["-march=rv32imafc", "-mabi=ilp32f"]);
    }

    #[test]
    fn riscv32_variants_map_to_their_abi() {
        let cases = [
            ("riscv32i-unknown-none-elf", "-march=rv32i", "-mabi=ilp32"),
            (
                "riscv32imac-unknown-none-elf",
                "-march=rv32imac",
                "-mabi=ilp32",
            ),
            (
                "riscv32imafdc-unknown-none-elf",
                "-march=rv32imafdc",
                "-mabi=ilp32d",
            ),
        ];
        for (target, march, mabi) in cases {
            let (_, args) = riscv_target_config(target).unwrap();
            assert_eq!(args, [march, mabi], "target {target}");
        }
    }

    #[test]
    fn non_riscv_targets_have_no_arch_override() {
        assert!(riscv_target_config("aarch64-apple-darwin").is_none());
        assert!(riscv_target_config("riscv64gc-unknown-none-elf").is_none());
        assert!(riscv_target_config("riscv32-unknown-none-elf").is_none());
        // Linux targets keep cc-rs's ABI choice.
        assert!(riscv_target_config("riscv32gc-unknown-linux-gnu").is_none());
    }

    /// Every optional widget lists the other optional widgets its C sources call
    /// (`lv_<widget>_create`, `_class`, `_set_*`, ...). Calls guarded by a
    /// separate option are listed in `GUARDED`.
    #[test]
    fn widget_requires_match_lvgl_sources() {
        // (widget, dependency) pairs reached only behind another `LV_USE_*`.
        const GUARDED: &[(&str, &str)] = &[("calendar", "dropdown")];
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/lvgl-9.6/src/widgets");
        let prefix = |name: &str| {
            match name {
                "animimage" => "animimg",
                "ime-pinyin" => "ime_pinyin",
                _ => name,
            }
            .to_string()
        };
        for widget in OPTIONAL_WIDGETS {
            let mut source = String::new();
            for entry in std::fs::read_dir(root.join(if widget.feature == "ime-pinyin" {
                "ime"
            } else {
                widget.feature
            }))
            .unwrap()
            {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|ext| ext == "c") {
                    source.push_str(&std::fs::read_to_string(path).unwrap());
                }
            }
            for other in OPTIONAL_WIDGETS {
                if other.feature == widget.feature
                    || GUARDED.contains(&(widget.feature, other.feature))
                {
                    continue;
                }
                let base = format!("lv_{}_", prefix(other.feature));
                let calls = source.match_indices(&base).any(|(at, _)| {
                    let before = source[..at].chars().next_back();
                    let rest = &source[at + base.len()..];
                    !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                        && ["create", "class", "set_", "get_", "add_", "is_"]
                            .iter()
                            .any(|suffix| rest.starts_with(suffix))
                });
                assert_eq!(
                    calls,
                    widget.requires.contains(&other.feature),
                    "{} -> {}",
                    widget.feature,
                    other.feature
                );
            }
        }
    }
}

#[test]
fn package_contains_widgets_demo_inputs() {
    use std::{collections::BTreeSet, path::Path, process::Command};

    fn collect_files(root: &Path, dir: &Path, files: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).expect("read widgets demo directory") {
            let entry = entry.expect("read widgets demo entry");
            let path = entry.path();
            if entry
                .file_type()
                .expect("read widgets demo file type")
                .is_dir()
            {
                collect_files(root, &path, files);
            } else if matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("c" | "h")
            ) {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                );
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut required = BTreeSet::from([
        "vendor/lvgl-9.6/demos/lv_demos.c".to_string(),
        "vendor/lvgl-9.6/demos/lv_demos.h".to_string(),
        "vendor/lvgl-9.6/demos/widgets/lv_demo_widgets.c".to_string(),
        "vendor/lvgl-9.6/demos/widgets/lv_demo_widgets.h".to_string(),
    ]);
    collect_files(
        root,
        &root.join("vendor/lvgl-9.6/demos/widgets"),
        &mut required,
    );

    // Listing does not build the package; give nested Cargo its own target
    // directory so this also works while the outer `cargo test` holds a lock.
    let target =
        std::env::temp_dir().join(format!("rs_lvgl_sys-package-list-{}", std::process::id()));
    let output = Command::new(env!("CARGO"))
        .current_dir(root)
        .args([
            "package",
            "--list",
            "--allow-dirty",
            "--offline",
            "--locked",
        ])
        .arg("--target-dir")
        .arg(&target)
        .output()
        .expect("run cargo package --list");
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("remove package-list target directory");
    }
    assert!(
        output.status.success(),
        "cargo package --list failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 package file list");
    let packaged: BTreeSet<&str> = stdout.lines().collect();
    let missing: Vec<_> = required
        .iter()
        .filter(|path| !packaged.contains(path.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "demo inputs missing from package: {missing:?}"
    );
}
