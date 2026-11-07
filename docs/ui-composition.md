# UI composition and controlled forms

`rs_lvgl_ui` 0.7.0 provides `child_if`, semantic value handlers, component slots,
`typed::...` and `binding`. Install it from crates.io with
`rs_lvgl_ui = "0.7.0"`. Existing root/prelude constructors still return
`View<ComponentAction>`. Version 0.7.0 additionally provides
controlled range bindings and editing options; see the
[capability guide](ui-capability-depth.md).

## Conditional children, lists and functions

`child_if(condition, || view)` appends one child in the true branch. The closure
runs once and can move or borrow local data. The false branch does not evaluate
it. Arrays, iterators, `child` and `children` preserve insertion order:

```rust
use rs_lvgl_ui::prelude::*;

fn device_list(ids: &[u64], show_help: bool) -> View<()> {
    column([label("Devices")])
        .children(ids.iter().map(|id| {
            button(format!("Device {id}")).key(Key::Id(*id)).on_click(())
        }))
        .child_if(show_help, || label("Select a device"))
        .gap(8)
}
```

Business keys identify list items across reorders and preserve viable native
focus. A helper is an ordinary function returning a description; the parent
passes state and callbacks and receives actions through `.map(...)`. Helpers
do not introduce independent state, subscriptions or a native wrapper object.

## Semantic events

| Handler | Native controls | Callback value |
| --- | --- | --- |
| `on_toggle` | Switch, checkbox | `bool` |
| `on_value_changed` | Single-value slider, bar, arc, spinbox | `i32` |
| `on_range_changed` | Two-ended range slider | `SliderRange { lower, upper }` |
| `on_selection_changed` | Dropdown, roller, tabs, tiles, button matrix, message-box footer | `Option<u32>` |
| `on_cell_selected` | Table | `Option<TableCell>` with `row` and `column` |
| `on_date_selected` | Calendar | `Option<CalendarDate>` |
| `on_text_changed` | Textarea | Callback-scoped `&str` |

`None` means a valid absence, such as no pressed button or a calendar day-name
cell. An invalid native representation emits no semantic action. Numeric legacy
handlers on erased `View` retain the documented packed/sentinel representation;
indices that cannot fit an `i32` are skipped. `table_cell` now decodes the full
packed unsigned axes, including rows above 32767; reserved axis sentinels mean
no cell.

Toggle, range, selection, cell, date and numeric handlers replace the **same value
handler** in call order. Text, click and generic-event handlers are independent.
Applications select one value representation per control. A mapper must not
unwind or reenter native UI operations; native callbacks enqueue the mapped
action and `Ui::drain` later calls `update`.

## Typed construction and the View boundary

`typed::switch(...)` returns `Node<ComponentAction, typed::kind::Switch>`.
Its styles, key, interaction modifiers, animations and `map` retain the widget
marker. `placeholder`, `password` and text handlers require a textarea marker;
roller properties require a roller marker. Wrong widget-specific calls fail
during Rust compilation. Shared style properties retain their existing native
part semantics and are not restricted to a particular widget family.

Single-child and component-slot methods accept typed nodes directly. A mixed
array, an iterator yielding a view, a component return or branches with different
widget types use explicit `.into_view()`:

```rust
use rs_lvgl_ui::prelude::*;

fn status(enabled: bool) -> View<()> {
    typed::column([
        typed::label("Power").into_view(),
        typed::switch(enabled).on_toggle(|_| ()).into_view(),
    ])
    .child(typed::button("Save").on_click(()))
    .child_if(enabled, || typed::label("Enabled"))
    .into_view()
}
```

`into_view()` moves the same descriptor without allocating or touching LVGL.
After erasure, the existing runtime property checks apply: diagnostics reports
ignored modifiers and `strict-properties` rejects them. Typed construction checks
widget capabilities; ranges, duplicate keys, resource lifetimes and native
allocation failures still need runtime validation. Optional typed constructors
and their markers use the same widget features as their ordinary counterparts.
No public construction macro or alternate state model is introduced.

## Explicit form bindings

`binding(current_value, action_mapper)` combines a controlled value and its
input handler. Typed numeric constructors accept `i32` or a numeric binding;
range_slider accepts `SliderRange` or a range binding;
switch/checkbox accept `bool` or a toggle binding; textarea accepts `String`,
`&String`, `&str` or the corresponding text binding.

```rust
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum FanSettingsAction {
    SpeedChanged(i32),
    EnabledChanged(bool),
    NameChanged(String),
}

fn fields(speed: i32, enabled: bool, name: &str) -> View<FanSettingsAction> {
    typed::column([])
        .child(typed::slider(0, 100, binding(speed, FanSettingsAction::SpeedChanged)))
        .child(typed::switch(binding(enabled, FanSettingsAction::EnabledChanged)))
        .child(typed::textarea(binding(name, |text: &str| {
            FanSettingsAction::NameChanged(text.into())
        })).placeholder("Device name"))
        .padding(12)
        .gap(12)
        .into_view()
}
```

Nested form containers reserve their own inner padding for native drawing.
Slider knobs extend beyond the track, especially at the range endpoints, and
focus outlines extend beyond a control's layout box. Ancestor padding and `gap`
do not prevent an inner zero-padding container from clipping those pixels. The
example uses 12 px with the bundled default widget styles; custom knob, shadow
or outline dimensions require corresponding space and native render checks.

The constructor consumes the binding, copies borrowed current text into the
existing descriptor, and installs one handler. The mapper is `'static`; it
cannot retain a short borrow of application state. Input does not assign to a
field or retain `&mut self`. The complete path remains:

```text
native input -> mapped Action -> queue -> update -> view -> reconcile
```

If native input changes a slider from 50 to 80 and `update` rejects it while the
model remains 50, reconciliation still restores 50. Programmatic correction
suppresses feedback events. `RenderDecision::Skip` is valid only when native
correction is unnecessary; model equality alone is insufficient. A later
`on_toggle`, `on_range_changed`, `on_value_changed` or `on_text_changed` replaces the binding's
corresponding handler and releases its capture.

The [runnable form](../.agents/skills/rs-lvgl-ui/references/form.rs) includes
component-specific actions, model validation, list composition and teardown.

## Current custom-widget inputs

`custom(widget, props)` delivers `CustomWidget::Props` on mount and each compatible
reconciliation. The same key and concrete widget type retain the native object.
The new factory is discarded after its properties transfer; equal properties are
still applied. `apply_props` and time-based `tick` run independently. A different
key or implementation type replaces the subtree. Props and widget backing survive
native deletion callbacks, including failed partial application. See the
[custom lifecycle](ui-capability-depth.md#custom-properties-animation-and-deletion).

## Component inputs and slots

| Component | Input and content contract |
| --- | --- |
| `ListItem` | `new(text)` supplies default text. `leading` replaces `icon`, `content` replaces the default text, and `trailing` replaces `detail`; later calls to the same slot replace earlier ones. |
| `Panel` | `header_action`/`header_actions` append header widgets. `content` replaces the body; later `child`/`children` append. `action` remains a compatibility spelling for `header_action`. |
| `Dialog` | `content` replaces the body; later `child`/`children` append. Footer `button`/`primary` actions remain separate from body content. |

Slots retain their own action type and can use `.map(ParentAction::Child)`.
Explicit user keys and styles survive. An unkeyed list slot receives a stable
slot key; default slot styling is applied below explicit caller fields. Slots
introduce no extra native container. An explicitly supplied list slot uses a
temporary boxed descriptor during composition, then moves it into the row.

## Allocation and verification

Typed markers are zero-sized, and erasure moves existing fields. A binding
uses the same value/text handler slot as explicit handlers, with no persistent
binding object or reactive graph. Closures, owned strings, child vectors,
component slots, `.map` and native LVGL storage can allocate.

`scripts/check-ui-api.py` compiles real `no_std` positive and negative consumers.
`tests/bindings.rs` verifies native rejection, queue timing, mapped actions,
identity and no correction echo. `scripts/check-ui-leaks.py` exercises repeated
conditional removal, typed forms, mapped handlers and slots in both allocation
domains. `scripts/check-ui-costs.py` compares equivalent erased, typed and bound
form construction and retains the existing style baseline comparison. Host
measurements do not establish device RAM, timing or linked firmware size.
