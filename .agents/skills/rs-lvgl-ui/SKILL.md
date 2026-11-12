---
name: rs-lvgl-ui
description: Build or modify declarative Rust screens with rs_lvgl_ui, including App::Action, styles, composition, controlled forms and resource lifetimes. Verify the resolved release or development source before choosing APIs.
---

# LVGL Rust UI

## Locate the API

Identify the application's Cargo dependency and the actual source checkout. This
skill targets rs_lvgl_ui 0.8.0, layered over LVGL 9.6. For an older release,
use the skill from its matching tag. Do not insert 0.8.0 names into an application
consuming an older API.
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
  `grid(columns, rows, children)` also takes its children directly. `container()` is
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

## Composition and forms

Version 0.8.0 includes lazy children, semantic callbacks, component slots, typed
construction and explicit bindings. The [runnable form](references/form.rs)
preserves those composition contracts in this checkout. Resolve the actual source
and declared widget features before choosing API names.

- `child_if(condition, || view)` evaluates its FnOnce closure only for true.
  Lists use iterators and stable business keys; functions accept props/state
  and return views. Compose child actions with `.map(ParentAction::Child)`.
- Prefer `on_toggle(bool)`, `on_selection_changed(Option<u32>)`,
  `on_cell_selected(Option<TableCell>)` and `on_date_selected(Option<CalendarDate>)`
  for the matching controls. Numbers use `on_value_changed(i32)`, text uses
  `on_text_changed(&str)`. The semantic methods and numeric method replace
  one shared value-handler slot; text/click/generic handlers are independent.
  None is valid absence; invalid native representations emit no semantic action.
- `typed::...` retains widget capabilities through common styles, keys and map.
  Use `.into_view()` for mixed arrays, iterator items, different branch types
  and View return boundaries. A single `.child(typed::switch(...))` or component
  slot accepts the typed node directly. Wrong widget-specific methods fail
  before erasure; erased views still use diagnostics/strict runtime checks.
- `typed::slider(0, 100, binding(current, Action::SpeedChanged))`, typed
  switch/checkbox bool bindings and textarea text bindings combine the current
  value with one action mapper. No mutable model borrow or automatic setter
  survives construction. Update accepts or rejects the input. Reconcile even
  when rejection leaves the model unchanged; RenderDecision::Skip would skip
  the necessary native correction. Programmatic correction suppresses feedback.
- ListItem leading/content/trailing retain keys, explicit styles and actions.
  Panel uses header_action/header_actions for header widgets. Panel/Dialog
  content replaces the body; later child/children append. Slots add no native
  container; explicit list slots temporarily box their descriptor.

Typed erasure adds no allocation. Bindings consume the existing event slot.
Owned text, capturing handlers, child vectors, map, slots and native storage
still have costs; measure Rust and LVGL separately for memory claims.

## Controls, charts and custom Props

The [runnable depth recipe](references/capability-depth.rs) uses the current
breaking API. Enable widget-slider/textarea/chart/arc and alloc for this recipe;
it also depends directly on matching rs_lvgl and rs_lvgl_sys for native readback.
Use the matching 0.8.0 packages in a registry consumer.

- Flex uses independent `.flex_wrap(bool)` and `.flex_reverse(bool)`; keys and
  descriptor child order are preserved. Reserve inner padding at both slider thumbs.
- Use `range_slider(min,max,SliderRange)` and `.on_range_changed`, or typed range
  construction with `binding(range, Action::Range)`. Endpoints are controlled;
  invalid ordering/domain arithmetic rejects without sorting or clamping. The
  range callback shares the value-handler slot; single sliders use numeric input.
- Textarea `accepted_chars` configures nonempty, NUL-free editing characters.
  `input_max_length(Option<NonZeroU32>)` uses scalar count, not UTF-8 bytes or
  grapheme clusters. `cursor_position(Option<u32>)` uses scalar boundaries;
  `cursor_click_position(bool)` controls pointer placement. Omission restores
  unrestricted input/no limit/pointer placement; an omitted cursor leaves native
  editing placement uncontrolled for unchanged model text. Full model assignment
  temporarily removes input filters and restores them afterwards. Filters must
  not truncate authoritative text. Font glyph coverage needs separate checks.
- Use only `chart(ChartSpec)` / typed::chart. Series have stable IDs, ChartData,
  independent X/Y axis selection and explicit ChartValue::Missing. Scatter
  requires XY, other forms Y; Curve needs effective native vector support.
  Bar rejects missing/unfilled history; Stacked rejects negative samples,
  mixed Y axes and cumulative overflow. Native extreme geometry arithmetic is
  not comprehensively audited. No old constructor or series adapter exists.
- ChartHistory has explicit epoch/capacity/kind/sequence. Snapshot construction
  copies retained samples; clones share immutable data. Metadata and actual
  overlap must prove append continuity. Discontinuities, capacity/mode changes
  and remount recover with full synchronization. A discarded view consumes no
  progress. Sequence exhaustion requires a new epoch; no reboot persistence.
- Implement CustomWidget::Props and `custom(widget, props)`. Pure validate runs
  before native mutation; create/apply_props run on mount. Same concrete type/key
  applies every incoming Props, even equal values, to the mounted instance; the
  unused incoming factory drops. A type/key change replaces it. Tick is separate.
  Successful apply retires old native references; failed partial apply retains
  both Props through deletion. Delete runs once while the root is live; widget
  and backing data survive native delete callbacks, then drop. Create error
  cleans up its own partial subtree. Hooks do not delete the root or reenter Ui;
  native callbacks must not unwind. Raw pointers retain their unsafe contracts.

## UI names, units and migration

Use container/progress_bar/message_box/animated_image; match typed constructors
and Container/ProgressBar/MessageBox/AnimatedImage markers. Use ScrollDirection
and scroll_direction with HORIZONTAL/VERTICAL; TextOverflow with Ellipsis and
ScrollLoop; Align::TopCenter/BottomCenter/CenterLeft/CenterRight. Old names have
no aliases in this source. The included [clarity example](references/api-clarity.rs)
compiles the 0.8.0 API.

Use Scale::percent(50) and Angle::degrees(90) for transforms and image scale.
Scale::ratio(n, d) is checked and rounds to native 1/256, ties upward;
Angle::tenths retains sub-degree precision. Native scale/angle overflow rejects
before mutation. Images reject zero scale; style transforms allow it. Rotation
and scale animations use Animation::rotation/scale_x/scale_y with typed endpoints.
Spinner sweep angles remain whole degrees.

Use justify_content(FlexAlign), align_items(ItemAlign), align_content(FlexAlign)
independently. Migrating old flex_align(main,cross) requires all three writes:
main, cross-as-ItemAlign, and cross-as-align_content. Floating also excludes
parent scrolling. There is no implicit CSS stretch or absolute-position contract.

Use thumb_style/thumb_state_style on sliders/switches and
progress_style/progress_state_style on sliders/progress bars. Other native parts
use NativePart and native_part_style/native_part_state_style. A later style for
the same part/state replaces the whole prior patch; combine fields in one patch.
Typed misuse fails capability bounds; erased misuse follows diagnostics/strict.
Color-only thumb patches preserve the native circular radius. A fixed pixel
radius changes that shape as the thumb size changes; compare the full native
outline when customizing geometry. For proportional scaling, set both transform
axes to the same Scale. Rotation/scaling changes drawn bounds without reserving
layout space; use a centered pivot and a containing slot that fits all corners,
as in the clarity recipe.

## Styles and input

View constructors initialize their own embedded style. Prefer direct modifiers:
`.gap(12).padding(16).width(percent(100)).height(content()).grow(1)`.
Dimensions use `px`, `percent` or `content`; spacing is in pixels.
`.font(theme.title_font).text_color(theme.text)` makes theme inputs explicit.
`style()` creates an unset reusable patch; `Style::default()` is only the Rust
value-construction primitive. Both leave properties unset and preserve inheritance.
`Style::plain()` explicitly writes plain-node base values; it is not an empty
patch or complete reset. Ordinary containers/labels already apply it. Extended
properties, offsets, floating, flex placement and inherited text remain unset. `Style::accent(theme)`
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
Each nested form container needs its own inner space: native slider knobs draw
outside the track, including at both range endpoints. Outer padding or sibling
gap does not protect pixels clipped by an inner zero-padding row/column. The
form recipe reserves 12 px for the bundled defaults; verify custom knob/shadow/
outline sizes and all four edges in actual native snapshots.

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
