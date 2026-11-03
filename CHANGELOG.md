# Changelog

## Unreleased

### Added

- Add an independent, default-disabled EmbedScope example using the published
  0.1.0 SDK and rs_lvgl 0.3.1. It exposes real native tree, display capture,
  pointer input, logs and metrics over loopback TCP, with process
  acceptance coverage and documented debug/release feature selection.

## 0.3.1 — 2026-11-02

### Changed

- Organize `rs_lvgl_ui` descriptors, constructors, themes, runtime, reconciliation,
  events and native widget adapters into functional modules. Share internal
  message adapters, validation, checked-state writes and event suppression while
  preserving the public API, feature gates and native behavior. Add a source map
  for library maintenance.

## 0.3.0 — 2026-11-01

### Breaking changes

- Make `NativeObject` an unsafe trait. External implementations must guarantee
  valid native object lifetimes and UI-thread access; migrate through a reviewed
  `unsafe impl`. Expand `Widget::from_raw` safety requirements. This does not add
  automatic object/display deletion or certify every existing FFI wrapper.

### Added

- Add explicit bounded queue outcomes, rejection/eviction policies, caller-defined
  tail merging and native-handler outcome observers without changing `Ui::post`.
- Add optional `diagnostics` path metadata and opt-in `strict-properties` checking
  for wrong-kind modifiers, preserving metadata through message mapping.
- Retain loaded RGB565 images and native font resources across mount, replacement,
  failure cleanup and unmount. Admission checks the effective synchronous software
  renderer configuration with image/header caches disabled. Font import remains
  an unsafe transfer requiring a matching destructor and stable backing data.
- Add defaulted `App::update_with_render` feedback, borrowed static label/button
  constructors and a stable-child-order reconciliation path. Add an isolated
  update-cost harness for cumulative Rust allocations, peak bytes and label setters.

### Documentation

- Document [runtime/resource contracts and migration](docs/ui-runtime.md),
  configuration limits and the validation/measurement entrypoints.
- Move the bilingual website, browser studio and playground to the independent
  [rs_lvgl_website](https://github.com/umbrella22/rs_lvgl_website) repository.
  Publish through its own GitHub Pages workflow at
  [the new documentation address](https://umbrella22.github.io/rs_lvgl_website/),
  with a pinned library source revision for native and WebAssembly builds.
  Update README, testing, studio and future release links.

### Fixed

- Detect failed native image source installation before releasing the previous
  descriptor/data. Decoder rejection now returns `InvalidProperty` and cleans the
  failed subtree; native string-allocation failure preserves prior data until
  cleanup. Loaded RGB565 images reject configurations that disable RGB565 drawing.

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
