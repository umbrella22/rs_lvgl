# Application inputs

This recipe requires the `rs_lvgl_sys` implementation from `rs_lvgl` with
`LVGL_CONFIG_PATH`, `LVGL_EXTRA_C_DIR` and `LVGL_EXTRA_HEADERS` readers. Locate that
source through the application's manifest; paths below refer to the application,
not to the installed skill directory.

Put a complete LVGL 9.6 configuration at `native/lv_conf.h`. Start from the matching
provider's configuration, then change the relevant values. Quoted relative includes
retain their meaning. Optional definitions can be factored into an included file:

```c
/* native/settings.h */
#define APP_DPI 173
```

Within `native/lv_conf.h`, set `LV_DPI_DEF` to `(APP_DPI + 0)` after including
`settings.h`. An existing definition can be replaced with `#undef` followed by
`#define`; final integer expressions drive both C and Rust.

```toml
# application/.cargo/config.toml
[env]
LVGL_CONFIG_PATH = { value = "native/lv_conf.h", relative = true }
LVGL_EXTRA_C_DIR = { value = "native", relative = true }
LVGL_EXTRA_HEADERS = { value = "native/application.h", relative = true }
```

Cargo resolves these relative values from the directory containing `.cargo`.
Direct environment values must be absolute existing UTF-8 paths. A multi-header
`LVGL_EXTRA_HEADERS` uses the OS path separator; use absolute entries for that list.
The directory is recursively scanned for `.c` files. Private headers and added or
removed source files also invalidate the build.

```c
/* native/application.h */
#include "lvgl.h"
int application_value(void);
/* Add this declaration when a matching generated font C file is present:
extern const lv_font_t application_font;
*/
```

```c
/* native/application.c */
#include "application.h"
int application_value(void) { return 7; }
```

A real raw consumer can now call `unsafe { rs_lvgl_sys::application_value() }` under
that function's contract. A generated font C file also belongs under `native/`;
add its declaration to the header passed to bindgen. Merely creating an empty
`lv_font_t` is not a valid rendered font.

The header is not rewritten. An explicit Cargo feature such as `widget-slider`
requires the corresponding effective capability; include native dependencies such
as `LV_USE_BAR`. For SDL, enable both `LV_USE_SDL` and the Cargo `sdl` feature.
For Lottie, configure internal ThorVG, vector/matrix/float support and the selected
widgets, and provide a target C++14 toolchain/runtime. Application C++ sources need
the application's own build integration.

For typed `rs_lvgl_ui` consumers, retain core object/label/button, Montserrat 14 and
all selected UI capabilities. Generated imperative wrappers can follow the linked
header, but handwritten UI APIs also depend on their Cargo feature declarations.

Check a changed included header in the same target directory and assert the
executable observes the new value. Restore an invalid header and verify recovery.
A packaged consumer must compile the unpacked crate rather than reaching into the
original source tree. Never use a successful host run as proof of another target's
ABI, renderer or allocator behavior.
