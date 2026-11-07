# Changelog

## 0.7.0 — 2026-11-07

### Breaking changes

- Replace chart construction with `chart(ChartSpec)` and stable `ChartSeries`
  descriptors for axes, presentations, cursors and owned samples/history. Remove
  the old constructor signature and series field layout without adapters.
- Replace custom widgets with associated `Props`, pure validation and distinct
  `apply_props`/`tick` hooks; use `custom(widget, props)`. Retain widget/Props backing
  through native deletion, including partial failure.
- Add an opaque optional input-settings field to the Textarea descriptor variant.
  Manual enum consumers must migrate; builders configure the payload.

### Added

- Add independent Flex wrapping/reversal, controlled two-ended sliders and typed
  range bindings/events. Add textarea input filters, optional character limits,
  scalar cursor positions and pointer-placement controls while preserving complete
  authoritative model assignment.
- Add bounded immutable chart snapshots with overlap-verified appends and full
  recovery after discontinuity/remount, five chart forms and independent axes.
- Add native Control depth and Chart depth Studio pages, a property-driven custom
  gauge, external API controls and unpacked-package native consumer validation.
- Add representative host workload/history experiments and complete routine CI
  correctness modes with corruption, clock and allocation controls.

### Changed

- Store optional textarea editing settings only in the Textarea payload. Comparable
  host fixtures recover eight bytes per View descriptor without extra allocations.
  This is a storage measurement, not a device frame-rate claim. Default features
  and native provider inputs remain unchanged. All four crates advance to 0.7.0;
  see the [migration guide](docs/ui-api-migration.md).

## 0.6.0 — 2026-11-06

### Breaking changes

- Unify declarative constructor, typed marker, scroll/overflow/placement, animation
  and style names. Remove replaced paths without aliases. Ordinary plain views
  no longer repeat their automatic base style; `Style::plain()` retains explicit
  base-patch behavior. See the [migration guide](docs/ui-api-migration.md).
- Require `Scale` and `Angle` for transform/image scale and rotation, with checked
  native ranges and explicit rounding; add typed rotation/scale animation endpoints.
- Replace coupled flex alignment with independent `justify_content`, `align_items`
  and `align_content`. Preserve floating layout and parent-scroll exclusion.
- Expose native selectors as `NativePart`/`native_part_style`; add capability-checked
  thumb/progress styles for their supported widgets and label text overflow.


### Fixed

- Preserve native circular slider/switch thumbs in the API clarity examples.
  Scale the transform sample equally on both axes around its center and reserve
  its rotated drawing bounds. Reserve full slider endpoint clearance and fit
  the standalone skill controls within a 320 × 240 display. Native contour
  comparisons cover both themes,
  control sizes, values and states, with deliberate distortion controls.

## 0.5.0 — 2026-11-06

### Added

- Add lazy `child_if`, semantic bool/optional-index/cell/date callbacks and
  ListItem leading/content slots, Panel header actions, and Panel/Dialog body
  replacement/append composition.
- Add optional `typed::...` constructors with compile-time widget capabilities,
  explicit `.into_view()` boundaries, and `binding(current, action_mapper)` for
  numeric, toggle and text controls. Existing erased constructors remain.
- Add external no_std API controls, native rejected-input correction coverage,
  conditional/slot lifecycle workloads and comparable form-construction costs.
  Update examples, composition guidance and source-aware skills together.

### Fixed

- Derive handwritten Flex/Grid API availability from the effective linked target
  configuration, including headers that disable either or both layouts.
- Decode legacy packed table cells with unsigned axes, preserving valid rows
  above 32767 while retaining reserved-axis absence sentinels.
- Reserve native knob and focus-outline drawing space in nested form examples
  and the settings footer; check complete native ink at slider endpoints.


## 0.4.0 — 2026-11-05

### Breaking changes

- Rename `rs_lvgl_ui::App::Msg` to `App::Action`, without a compatibility member.
  Implement applications with `type Action = ComponentAction`, use
  `Self::Action` in projections and `App<Action = ComponentAction>` in bounds.
  Examples use component-specific action types and `action` parameters. See
  [application action types](docs/ui-styles.md#application-action-type).
- Rename `rs_lvgl_ui::Style` builders `bg`, `bg_opacity`, `fg`, `pad`,
  `pad_left/right/top/bottom` and `flex_grow` to `background`,
  `background_opacity`, `text_color`, `padding`, `padding_left/right/top/bottom`
  and `grow`, without compatibility aliases. Conflicting getters use
  `background_value`, `background_opacity_value`, `text_color_value`,
  `padding_left/right/top/bottom_value` and `grow_value`. See
  [style contracts](docs/ui-styles.md).

### Added

- Add flat layout/appearance modifiers to views, `px`/`percent`/`content` size
  helpers and `style()` for reusable unset patches. Constructors own style
  initialization; modifiers preserve explicit-field precedence, inheritance
  and existing part/state behavior without adding descriptor storage.
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
