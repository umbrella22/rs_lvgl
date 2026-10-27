# rs_lvgl

**English** · [简体中文](https://github.com/umbrella22/rs_lvgl/blob/main/README.zh.md)

[Documentation](https://umbrella22.github.io/rs_lvgl_website/en/) · [Getting started](https://umbrella22.github.io/rs_lvgl_website/en/guide/getting-started.html) · [Component preview](https://umbrella22.github.io/rs_lvgl_website/en/studio.html) · [Playground](https://umbrella22.github.io/rs_lvgl_website/en/playground.html)

Rust bindings and declarative UI for [LVGL](https://lvgl.io/) 9.6.

Use LVGL widgets directly from Rust, or build screens with `rs_lvgl_ui` and
update them through messages. Run your interface on the desktop with SDL2,
then connect it to your device's display and input drivers.

![rs_lvgl component studio](https://raw.githubusercontent.com/umbrella22/rs_lvgl/main/docs/images/studio-overview.png)

## Try the demo

The [component preview](https://umbrella22.github.io/rs_lvgl_website/en/studio.html) runs
the same Rust/LVGL views in WebAssembly, including input and animations. In the
[playground](https://umbrella22.github.io/rs_lvgl_website/en/playground.html), edit an
example and interact with the result directly in your browser.

The component studio includes controls, charts, text input, layouts, media,
and navigation, with light and dark themes.
[Browse the gallery](https://github.com/umbrella22/rs_lvgl/blob/main/docs/component-studio.md) or explore the
[smaller examples](https://github.com/umbrella22/rs_lvgl/blob/main/examples/README.md).

You'll need Rust, a C/C++ compiler, libclang, SDL2, and pkg-config.

```sh
# Debian / Ubuntu
sudo apt-get install build-essential clang libclang-dev pkg-config libsdl2-dev

# macOS, with Xcode Command Line Tools installed
brew install sdl2 pkg-config
```

From the repository root:

```sh
cargo run -p rs_lvgl_ui --example studio --features showcase
```

## Use in your project

| Crate | Purpose |
| --- | --- |
| [`rs_lvgl_ui`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md) | Declarative screens, typed events, themes, and reusable components |
| [`rs_lvgl`](https://github.com/umbrella22/rs_lvgl/tree/main/rs_lvgl) | Rust wrappers for widgets, displays, input, styles, and timers |
| [`rs_lvgl_sys`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/README.md) | Raw LVGL bindings, native configuration, custom C sources, and fonts |
| [`rs_lvgl_codegen`](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_codegen/README.md) | Widget wrapper generation used by the build |

Add the crates.io dependencies to your project:

```toml
[dependencies]
rs_lvgl = { version = "0.2", default-features = false, features = ["alloc"] }
rs_lvgl_ui = "0.2"
```

`rs_lvgl` and `rs_lvgl_ui` support `no_std`. Embedded applications using the UI
layer need an allocator and platform-specific display and input drivers.

## Define a screen

An `App` holds your state. Its `view` builds a `View<Msg>` with ordinary
functions; input sends messages
to `update`. `Ui` applies those changes to the LVGL widgets.

```rust
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum Msg {
    Increment,
}

struct Counter {
    value: u32,
}

impl App for Counter {
    type Msg = Msg;

    fn update(&mut self, _: Msg) {
        self.value += 1;
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        column([
            label(format!("Count: {}", self.value)),
            button("Add one")
                .key(Key::Named("increment"))
                .style(Style::accent(theme))
                .on_click(Msg::Increment),
        ])
        .style(Style::default().gap(12))
    }
}
```

See the [complete counter example](https://github.com/umbrella22/rs_lvgl/blob/main/.agents/skills/rs-lvgl-ui/references/counter.rs)
for display setup and the event loop, or the [UI guide](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md)
for layouts, styling, and events.

## Choose widgets and fonts

Disable the default feature set to select the widgets and fonts you need:

```toml
[dependencies.rs_lvgl_ui]
version = "0.2"
default-features = false
features = [
    "alloc", "components", "widget-slider", "widget-switch", "font-montserrat-16"
]
```

Labels, buttons, and row, column, and grid layouts are always available.
QR codes, barcodes, GIF, Lottie, Pinyin input, and CJK fonts are optional.
See [feature selection](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md#choosing-widgets) for the full list.

## Documentation

The guides and GitHub Pages deployment are maintained in [rs_lvgl_website](https://github.com/umbrella22/rs_lvgl_website).

- [Usage guide (Chinese)](https://umbrella22.github.io/rs_lvgl_website/) · [English](https://umbrella22.github.io/rs_lvgl_website/en/) — installation, declarative UI, components, input, native integration and development checks.
- [UI guide](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_ui/README.md) — components, state, styling, and input.
- [Native integration](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/README.md) — `lv_conf.h`, custom C sources, and fonts.
- [Testing](https://github.com/umbrella22/rs_lvgl/blob/main/docs/testing.md) — test commands and simulator checks.
- [Changelog](https://github.com/umbrella22/rs_lvgl/blob/main/CHANGELOG.md).

## AI coding assistants

The included `rs-lvgl-ui` and `rs-lvgl-native` skills provide API examples and
integration guidance for coding assistants. Install them with:

```sh
python3 scripts/install-skills.py --dest ~/.agents/skills
```

See [skill setup](https://github.com/umbrella22/rs_lvgl/blob/main/docs/ai-skills.md) for project-local installation and usage.

## License

Rust code is licensed under [MIT](https://github.com/umbrella22/rs_lvgl/blob/main/LICENSE), with credits to Rafael Carício,
Nia Espera, and the other contributors to the original bindings. LVGL, ThorVG,
and other bundled dependencies retain their own licenses.
