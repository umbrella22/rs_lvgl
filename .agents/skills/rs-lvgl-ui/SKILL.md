---
name: rs-lvgl-ui
description: Build or modify declarative Rust screens with rs_lvgl_ui 0.4.0, including App::Action, flat styles, features, events and resource lifetimes. Verify the resolved dependency before applying this skill to another LVGL Rust crate.
---

# LVGL Rust UI

## Locate the API

Identify the application's Cargo dependency and the actual source checkout. This
skill targets `rs_lvgl_ui` 0.4.0 from `umbrella22/rs_lvgl`, layered over LVGL 9.6.
Read the resolved manifests and relevant APIs before assuming a symbol or feature
exists. Other LVGL wrappers have different APIs. In this source repository, the main
entry points are `rs_lvgl_ui/src/{constructors,app,node,theme}/mod.rs`,
`rs_lvgl_ui/src/prelude.rs` and `rs_lvgl_ui/README.md`. The functional module map
is in `docs/ui-source.md`.

Use ordinary functions, `View<M>` and `rs_lvgl_ui::prelude::*` for UI construction.
The included counter demonstrates the same API. Resolve the application's actual
dependency and feature set before using optional widgets or platform integrations.

Respect the target workspace's agent guide and the user's requested scope.
Skill installation does not authorize publication, hardware flashing or a broad
wrapper migration. Preserve other work in the checkout.

## Build the application behavior

- Model inputs as a component-specific `Clone` action enum such as
  `CounterAction`; use `type Action = CounterAction` and return a pure
  `View<CounterAction>` from `App::view`. State fields and `update` implement
  the behavior; action variants are inputs, not method/variable declarations.
  Name the update parameter `action`, or use a more specific name such as
  `navigation_action`; unused parameters may use `_`. C callbacks enqueue messages.
  Call `Ui::drain` outside native callbacks to update and reconcile once per batch.
- Import `rs_lvgl_ui::prelude::*` or explicit root functions/types. Build with
  `column([label("Title"), button("Save").on_click(SettingsAction::Save)])`.
  `row(children)` and `column(children)` accept arrays or iterators;
  `grid(columns, rows, children)` also takes its children directly. `obj()` is
  the plain container; native tabview/tileview use `.child`/`.children` for their
  pages. Append builders remain useful for conditional or incremental assembly.
  `View<M>` aliases the existing descriptor type; associated constructors such
  as `Node::column()` and `View::column()` are removed. Use ordinary functions,
  including `canvas`, media constructors, `custom` and unsafe `raw`.
- Use stable `Key::Named` or `Key::Id` values for dynamic siblings. Keys are
  unique among siblings; `Key::Auto` matches by position. Reordering a keyed
  list should preserve the native object and focus where it remains viable.
- State changed outside the message loop needs `Ui::update` or `Ui::mark_dirty`.
  An idle `drain` does not rebuild the view. Distinguish native timers from
  custom-widget animation advanced by `Ui::tick`.
- Check feature gates before choosing controls. Core nodes include object,
  row/column/grid, label and button. Other widgets and fonts have additive
  features; `components` gates the composite controls. QR/barcode/GIF/Lottie,
  Pinyin and CJK fonts are optional even with `widgets-all`.
- For overlays, return a tree from `App::overlay`; dialogs are state-driven.
  `components::dialog::Dialog` is a module import, not a root re-export.

Use [the small lifecycle example](references/counter.rs) for a complete runnable
memory-display application. Adapt its state and view to the requested product;
a memory display does not provide a visible simulator or board driver.

## Styles and input

View constructors initialize their own embedded style. Prefer direct modifiers:
`.gap(12).padding(16).width(percent(100)).height(content()).grow(1)`.
Dimensions use `px`, `percent` or `content`; spacing is in pixels.
`.font(theme.title_font).text_color(theme.text)` makes theme inputs explicit.
`style()` creates an unset reusable patch; `Style::default()` is only the Rust
value-construction primitive. Both leave properties unset and preserve inheritance.
`Style::neutral()` explicitly clears the container defaults. `Style::accent(theme)`
provides a flat themed action button. Native defaults can include shadows; set
shadow width/color/opacity/offset explicitly when the design needs elevation.
Flat modifiers and `.style(patch)` follow call order: the last explicit field
wins, including zero/false. Unset fields keep prior values or inheritance.
`background` sets color and opacity 255; call `background_opacity` afterwards
for translucency. Style property getters use a `_value` suffix.
Theme presets copy tokens; `Ui::set_theme` requests a new view on the next
mount/update/drain. A cached old patch is not a live theme binding.
Flat setters update the embedded style without per-property heap allocations;
children, owned text and reconciliation can still allocate.
Use `.state_style(State::PRESSED, style().opacity(200))`, `FOCUSED` and `FOCUS_KEY` for interaction
feedback. Leave enough parent padding for an outer outline to avoid clipping.

`input-keypad` enables APIs without activating focus. Use `Ui::enable_focus` or
bind a keypad/encoder explicitly. Modal overlays scope that UI's focus and restore
viable prior focus on dismissal. Do not enable focus implicitly for an existing
touch-only screen. Direct event handlers are local; descendant observation uses
explicit bubbled handlers and native bubbling flags.

## Retention and teardown

All LVGL access is on one UI thread. Keep displays alive while any UI controller,
input, object, callback or retained resource uses them. Unmount controllers and
release inputs first; finish rendering/flush/DMA/GPU work before unsafe native
display deletion or deinitialization. Dropping a `Display` handle does not delete
its native display. Retained font/image pointers need their full native lifetime.
`Snapshot` contains copied RGBA pixels and can survive unmount.

Prefer typed views. A raw/custom escape requires local pointer and lifetime
justification; do not turn a raw-pointer forwarding API into a safe API merely
by moving its unsafe block.

## Optional remote diagnostics

Install host tools from [EmbedScope Releases](https://github.com/umbrella22/EmbedScope/releases/latest): use the platform CLI ZIP and add its directory to PATH, or open the portable Inspector. Cargo installation is optional for existing Rust environments.

Use [the EmbedScope recipe](references/embedscope.md) for an application-owned, default-disabled remote Inspector connection. `diagnostics` only supplies UI events/sink; it starts no Agent or transport. Debug/release profiles do not enable or disable Cargo features. Ship without the application's diagnostic feature and verify its unified dependency graph and installed image.

## Verify the changed contract

In this repository, use serial native tests and the examples matching the changed
feature set. Reduced-feature compilation catches accidental optional dependencies.
`python3 scripts/check-ui-api.py` compiles external `no_std` consumers and checks
root/prelude capability gates, removed constructors and the unsafe raw boundary.
For lifecycle changes, `python3 scripts/check-ui-leaks.py` measures Rust and native
allocation balance with deliberate leak controls. For example rendering, use
`python3 scripts/check-ui-examples.py`; native SDL smoke and snapshot assertions
are distinct from actual OS-input or physical-board validation.

In a downstream application, run its own manifest, selected features and display
path. Check the target runner before any cross-target run; an `espflash` runner
writes hardware. Report actual checks and missing target/display evidence.
