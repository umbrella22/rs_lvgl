# Changelog

## Unreleased

### Documentation

- Move the bilingual website, browser studio and playground to the independent
  [rs_lvgl_website](https://github.com/umbrella22/rs_lvgl_website) repository.
  Publish through its own GitHub Pages workflow at
  [the new documentation address](https://umbrella22.github.io/rs_lvgl_website/),
  with a pinned library source revision for native and WebAssembly builds.
  Update README, testing, studio and future release links.

## 0.2.0 — 2026-10-27

### Breaking changes

- Replace associated `Node::...` UI constructors with ordinary functions exported
  from the crate root and `prelude`. `row(children)`, `column(children)` and
  `grid(columns, rows, children)` accept child collections directly. Existing
  view modifiers and incremental `.child`/`.children` composition remain available.
- Export `View<M>` as the descriptor alias used by `App` and composite components.
  Migrate `Node::button("Save")` to `button("Save")`, and import
  `rs_lvgl_ui::prelude::*` for the common functions and types.

### Documentation

- Publish bilingual Rspress guides on GitHub Pages, with crates.io installation,
  embedded device integration, feature selection and application lifecycle guides.
  Add documentation, quickstart, component preview and playground links to both
  READMEs.
- Add a WebAssembly component preview built from the desktop studio's Rust/LVGL
  views, with nine categories, pointer and keyboard input, media and animation.
- Add an editable code/preview playground that runs supported Rust UI code on
  static hosting through a precompiled interpreter and real LVGL. Include stateful
  controls, source diagnostics, execution limits and last-successful previews.
- Provide optional local Rust compilation for complete applications. Document
  the browser syntax boundary and keep compiler settings in the full playground.
- Pair shared Rust examples with native renders, and explain state, message flow,
  native builds and capabilities using interactive diagrams that follow the site
  theme. Add responsive navigation, resizable playground panes and source copying.
- Validate translated pages, static links, deployment subpaths and generated
  example/runtime identities in CI before deploying GitHub Pages.

## 0.1.0

- Rust bindings for LVGL 9.6, with raw, widget, code-generation and declarative UI crates.
- Typed components, message-driven state, themes, keyed reconciliation, input and animation.
- Optional Canvas, QR/barcodes, GIF, Lottie, Pinyin input and CJK fonts.
- Component studio and focused declarative examples using the native SDL2 simulator.
- Application-supplied LVGL configuration, custom C sources and fonts.
- Native integration, resource lifecycle and Rust/LVGL heap checks.
- English and Chinese entry documentation, plus installable AI coding skills.
- Tagged crates.io releases for all four crates, with packaged and registry consumer checks.
