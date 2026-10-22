---
name: rs-lvgl-ui
description: Build or modify declarative Rust screens using this repository's rs_lvgl_ui App, Node and Ui APIs, including features, events, styling and resource lifetimes. Use for rs_lvgl applications; verify the dependency before applying this API to another LVGL Rust crate.
---

# LVGL Rust UI

## Locate the API

Identify the application's Cargo dependency and the actual source checkout. This
skill targets `rs_lvgl_ui` from `umbrella22/rs_lvgl`, layered over LVGL 9.6.
The local development API is not interchangeable with older registry `lvgl`,
`lightvgl-sys` or C++ wrappers. Read the resolved manifests and relevant APIs
before assuming a symbol or feature exists. In this source repository, the main
entry points are `rs_lvgl_ui/src/{app,node,theme}.rs` and `rs_lvgl_ui/README.md`.

Respect the target workspace's agent guide and the user's requested scope.
Skill installation does not authorize publication, hardware flashing or a broad
wrapper migration. Preserve other work in the checkout.

## Build the application behavior

- Model changes as a `Clone` message enum and `App::update`; make `App::view`
  return a pure `Node<Msg>` description. C callbacks enqueue messages. Call
  `Ui::drain` outside native callbacks to update and reconcile once per batch.
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

`Style::default()` leaves properties unset and preserves native inheritance.
`Style::neutral()` explicitly clears the container defaults. `Style::accent(theme)`
provides a flat themed action button. Native defaults can include shadows; set
shadow width/color/opacity/offset explicitly when the design needs elevation.
Use `.state_style(State::PRESSED, ...)`, `FOCUSED` and `FOCUS_KEY` for interaction
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

Prefer typed nodes. A raw/custom escape requires local pointer and lifetime
justification; do not turn a raw-pointer forwarding API into a safe API merely
by moving its unsafe block.

## Verify the changed contract

In this repository, use serial native tests and the examples matching the changed
feature set. Reduced-feature compilation catches accidental optional dependencies.
For lifecycle changes, `python3 scripts/check-ui-leaks.py` measures Rust and native
allocation balance with deliberate leak controls. For example rendering, use
`python3 scripts/check-ui-examples.py`; native SDL smoke and snapshot assertions
are distinct from actual OS-input or physical-board validation.

In a downstream application, run its own manifest, selected features and display
path. Check the target runner before any cross-target run; an `espflash` runner
writes hardware. Report actual checks and missing target/display evidence.
