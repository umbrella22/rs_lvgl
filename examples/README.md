# Examples

Every example in this directory uses `rs_lvgl_ui` components and the native SDL2
backend. Run from the repository root after installing the
[desktop prerequisites](../README.md#try-the-demo).

```sh
cargo run -p rs_lvgl_ui --example button_click --features simulator
```

Replace `button_click` with a name from the table:

| Example | Behavior |
| --- | --- |
| `app` | Switch and save action backed by application state |
| `button_click` | Click counter and message updates |
| `bar` | Controlled progress bar with advance/reset actions |
| `arc` | Interactive dial and shared numeric value |
| `scale` | Numbered scale and controlled slider |
| `animation` | Reversing native node animation with pause/resume |
| `demo` | Watch face, daily steps and progress |
| `sdl` | Native mouse input with increment/decrement actions |
| `sdl_demo` | Composite panel with slider, switch and progress |
| `rust_timer` | Rust monotonic clock and a pausable elapsed-time display |

The clock example additionally needs its feature:

```sh
cargo run -p rs_lvgl_ui --example rust_timer --features simulator,rust_timer
```

`EXAMPLE_FRAMES=100` gives a bounded run. `EXAMPLE_DARK=1` selects the dark theme;
`EXAMPLE_CAPTURE_DIR=target/captures` writes lossless native screenshots.
`support/` contains the shared display/event-loop setup and native view checks.
Each sample keeps its application state, messages and component tree in one file.

## Galleries

The [component studio](../docs/component-studio.md) covers the complete component
surface. Other galleries live in `rs_lvgl_ui/examples`:

```sh
cargo run -p rs_lvgl_ui --example studio --features showcase
cargo run -p rs_lvgl_ui --example gallery --features simulator
cargo run -p rs_lvgl_ui --example components --features simulator
cargo run -p rs_lvgl_ui --example input --features simulator,input-keypad
cargo run -p rs_lvgl_ui --example widgets_demo --features simulator
cargo run -p rs_lvgl_ui --example capabilities --features capability-gallery
```

## Checks

```sh
python3 scripts/check-ui-examples.py
```

On headless Linux, prefix the command with `xvfb-run -a`. The runner tests actual
views, sends native pointer events, checks layout and button shadows, and runs
all ten focused examples plus the galleries in real SDL windows. Logs and render
diagnostics go to `target/ui-audit`. See [testing](../docs/testing.md) for the
remaining library and native integration checks.
