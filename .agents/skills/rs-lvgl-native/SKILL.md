---
name: rs-lvgl-native
description: Integrate or diagnose rs_lvgl_sys from rs_lvgl with application lv_conf.h, custom C sources and fonts, feature projection and native rebuilds. Use for this vendored LVGL 9.6 provider, not as an external prebuilt LVGL or lightvgl-sys integration recipe.
---

# LVGL Rust Native Integration

## Identify the provider

Locate the resolved `rs_lvgl_sys` source and Cargo feature graph. This skill targets
the vendored LVGL 9.6 provider in `umbrella22/rs_lvgl`; other crates with
similar names have different configuration and ABI contracts. The source inputs
are `rs_lvgl_sys/build.rs`, `build_support/inputs.rs`, `vendor/include-9.6/lv_conf.h`
and `shims/`. Generated `bindings.rs` belongs to `OUT_DIR` and is not an edit target.

The build always compiles vendored LVGL. Do not add a second provider from an SDK,
assume a matching version proves ABI compatibility, or silently fall back from a
requested provider. External prebuilt linkage needs a separate design.

## Choose the narrow integration

For raw C calls, depend only on `rs_lvgl_sys`. Add `rs_lvgl` for imperative wrappers or
`rs_lvgl_ui` for typed composition only when the application needs them. Check whether
the checkout implementation is available through the chosen dependency; verify the
selected rs_lvgl release before using its UI API.

Use [the application-input recipe](references/application-inputs.md) when supplying
a configuration, C source or font. The header is authoritative. Cargo's explicit
widget/font/extension capabilities must agree with it; `minimal-config` does not
subtract capabilities from an application-owned header. Without an application
header, preserve the current vendored configuration and additive feature behavior.

## Trace configuration across the build

C compilation, target bindgen and the Rust public API must use the same effective
header, defines, alignment and target ABI. `rs_lvgl_sys` is both a normal and a build
dependency of `rs_lvgl`; those feature sets can differ. The wrapper generator must use
`DEP_LVGL_BINDINGS` and capability metadata from the linked normal dependency,
not a host build artifact found by scanning target directories.

Cargo watches application headers, transitive includes, extra C directories and
configuration environment inputs. Generated headers remain in their own `OUT_DIR`
and must not become new watched inputs. Rebuild tests need changed inputs and
same-directory recovery, not just a clean successful compilation.

For custom C functions, keep the unsafe boundary and document validity, retention
and callback unwinding. For fonts, use complete LVGL 9.6 descriptors with valid
static tables and callbacks; `Font::from_raw` cannot validate those obligations.
Native allocations must be released by their matching allocator. Default CLIB
monitor zeros do not establish leak freedom.

## Validate the actual consumer

In this repository:

```sh
cargo test -p rs_lvgl_sys --test build_script -- --test-threads=1
python3 scripts/check-native-inputs.py
```

The native runner needs Python 3.12+, a native C compiler, libclang, Cargo and POSIX
process groups. It runs raw and typed consumers, private/transitive header edits,
C/font changes, conflicts, interruption/recovery, output isolation and an unpacked
crate. For lifecycle changes, also run the isolated double-heap suite through
`python3 scripts/check-ui-leaks.py`.

For a downstream project, verify its actual header, toolchain, native symbols and
feature graph. Use a dedicated target directory for experiments. Inspect runner
configuration before cross-target execution; hardware flashing, publishing and
remote changes require their own authorization. Keep dependency/tool absence,
compile success, native execution and physical-target acceptance distinct.
