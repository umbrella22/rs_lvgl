# rs_lvgl_sys

Independent `no_std` raw bindings for vendored LVGL 9.6.0. This crate compiles LVGL
C sources and shims, then generates Rust declarations with bindgen/libclang.
It can be used without `rs_lvgl`, `rs_lvgl_ui` or a Rust UI runtime.

```toml
[dependencies]
rs_lvgl_sys = { version = "0.2" }
```

A C compiler and libclang are required. SDL and the C++ ThorVG provider are optional capabilities.

## Default configuration

With no application inputs, the build uses
[`vendor/include-9.6/lv_conf.h`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/vendor/include-9.6/lv_conf.h). Cargo widget/font
features patch a copy in `OUT_DIR`; the checked-in header is never rewritten.
The default allocator is CLIB and SDL is disabled.

`minimal-config` selects core objects, label, button and Montserrat 14; explicit
`widget-*` and `font-montserrat-*` features add capabilities and their native
requirements. Without `minimal-config`, features add to the vendored defaults.
`lv-demo-widgets` needs the full set and overrides `minimal-config` with a warning
on this default configuration path.

## Application configuration

An application can supply a complete LVGL 9.6 configuration without editing vendor
files. Put its inputs in the application workspace, for example:

```text
application/
  .cargo/config.toml
  Cargo.toml
  native/
    lv_conf.h
    display_settings.h
    application.h
    application.c
    fonts/application_font.c
```

Configure Cargo at that workspace root:

```toml
# .cargo/config.toml; values resolve from the directory containing .cargo.
[env]
LVGL_CONFIG_PATH = { value = "native/lv_conf.h", relative = true }
LVGL_EXTRA_C_DIR = { value = "native", relative = true }
LVGL_EXTRA_HEADERS = { value = "native/application.h", relative = true }
```

Start `native/lv_conf.h` from the pinned LVGL 9.6 configuration and adapt it to the
application. Its quoted includes resolve relative to the original header:

```c
/* Inside native/lv_conf.h */
#include "display_settings.h"
#define LV_DPI_DEF (APP_DPI + 0)
```

| Input | Contract |
| --- | --- |
| `LVGL_CONFIG_PATH` | Absolute existing UTF-8 header path; any filename is accepted. The header is authoritative and is not patched. |
| `LVGL_EXTRA_C_DIR` | Absolute existing directory. Recursively compiles `.c` files, including generated font sources. The directory and its private headers are tracked for rebuilds. |
| `LVGL_EXTRA_HEADERS` | Absolute existing header paths separated by the OS path separator (`:` on Unix, `;` on Windows). Bindgen includes their declarations, including names outside `lv_*`. For multiple headers, set an absolute path list in the environment. |
| `LVGL_CFLAGS` | Additional comma-separated `NAME=VALUE` preprocessor definitions, shared by C and bindgen. Not a general compiler-options string. |
| `LVGL_SYSROOT` | Existing cross-build input for target headers used by bindgen. The target C/C++ compiler must be configured separately. |

Relative environment paths, nonexistent paths, empty path entries and paths that
cannot be represented in a C include fail explicitly. Environment variables take
precedence over Cargo `[env]` values unless the application uses Cargo's `force`
setting. Application headers and their transitive includes are tracked, including
changes to numeric expressions and `#undef` / redefinitions. Generated integer
constants retain the default bindgen `u32`/`u64`/`i32`/`i64` policy.

When `LVGL_CONFIG_PATH` is set, `minimal-config` does not modify the application
header. Explicitly requested widget/font/extension capabilities must already be
enabled in its effective macros or the build fails. Native widget dependencies
must also be valid in the supplied header. SDL must agree in both directions:
the header enables `LV_USE_SDL` and Cargo enables `sdl` for library discovery/linking.

`rs_lvgl` reads the linked dependency's effective widget/font metadata when generating
wrappers. `rs_lvgl_ui` also has handwritten APIs and feature gates: its application
configuration must include the core widgets, Montserrat 14 and each selected UI
capability. A raw configuration that disables these is not a supported UI configuration.

## Custom C and fonts

Expose declarations from an explicit header:

```c
/* native/application.h */
#include "lvgl.h"
int application_value(void);
extern const lv_font_t application_font;
```

Compile implementations and generated LVGL 9.6 font `.c` files from
`LVGL_EXTRA_C_DIR`. Font assets and referenced tables must remain valid for every
native user. The generated Rust bindings include `application_value` and
`application_font`. A raw caller still observes the C safety contract:

```rust
// The application supplies a valid, immutable LVGL font with static tables.
// LVGL retains this descriptor while any object uses its style.
let font = unsafe {
    rs_lvgl_ui::Font::from_raw(core::ptr::addr_of!(rs_lvgl_sys::application_font))
};
```

This optional font adapter additionally needs `rs_lvgl_ui`; raw callers can use the
same descriptor directly. Extra source compilation currently covers C, not an
arbitrary application C++ build system.

## Native providers and extensions

This build always compiles the vendored LVGL provider. It does not discover or
link an external prebuilt LVGL library, including one supplied by a platform SDK.
A matching version string or header does not establish binary ABI compatibility.

`widget-qrcode`, `widget-barcode`, `widget-gif`, `widget-lottie`,
`widget-ime-pinyin`, `gridnav`, `font-cjk-14` and `font-cjk-16` are opt-in.
The default configuration path closes their native dependencies. Lottie selects
Canvas/image, vector graphics, matrices, floating-point coordinates and internal
ThorVG. Applications supplying a header must select compatible settings themselves.

ThorVG uses C++14 and the target C++ runtime. Its canonical
[configuration](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/shims/thorvg_config.h) is generated in `OUT_DIR` and preincluded;
no ignored Meson header is required. The checked Lottie shim reports loading,
timing and first-frame errors that the upstream void setter cannot return.
Vendored source provenance and licenses are retained.

## Build outputs and verification

```text
application header OR patched vendored header
  + vendored C/shims + application C/fonts
  -> target native library + bindings.rs
  -> DEP_LVGL_WIDGETS / FONTS / OPTIONAL_WIDGETS / BINDINGS
  -> optional lvgl generated wrappers
```

Configuration projections stay under the appropriate `OUT_DIR`. Cargo watches
source headers, included files, extra-source directories and relevant environment
inputs; it does not watch its own generated header as a new input.

From the repository root:

```sh
cargo test -p rs_lvgl_sys --test build_script -- --test-threads=1
python3 scripts/check-native-inputs.py
```

The runner executes raw custom-config/C/font consumers, tests enabled and disabled
Rust wrappers, mutates included/private headers, retries failed/interrupted builds,
checks independent output directories, and builds a consumer of an unpacked crate.
It requires Python 3.12+ and POSIX process groups. These checks establish host build
behavior; physical targets and external-provider ABI need separate acceptance.
