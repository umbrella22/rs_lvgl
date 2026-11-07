# UI controls, chart history and custom properties

These APIs are available in rs_lvgl_ui 0.7.0. Chart and custom-widget
construction have breaking signatures relative to 0.6.0.
The [migration guide](ui-api-migration.md) records the replacements.

All examples use one UI thread, `no_std` plus `alloc`, and the effective vendored
LVGL configuration. Widget features are additive. `widget-slider`,
`widget-textarea` and `widget-chart` expose their corresponding descriptions,
constructors, typed methods and events. Disabled features remove those public
surfaces. No new default feature or provider is required.

## Flex flow and controlled range sliders

`row(...).flex_wrap(true).flex_reverse(true)` selects native wrapping and reverse
flow independently. Both default to false and retain the descriptors' child order
and stable keys. Reversal changes native placement, not message order or identity.
`justify_content`, `align_items` and `align_content` still control independent
main, cross and track placement.

`range_slider(min, max, SliderRange::new(lower, upper))` uses the existing native
slider in two-ended mode. `.on_range_changed` receives one `SliderRange` containing
both endpoints. The typed constructor also accepts a binding:

```rust
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum Action { Range(SliderRange) }

fn range_field(current: SliderRange) -> View<Action> {
    typed::range_slider(0, 100, binding(current, Action::Range))
        .key(Key::Named("limits"))
        .width(px(180))
        .into_view()
}
```

The constructor does not sort or clamp endpoints. Rendering rejects unordered
endpoints, values outside the domain and unrepresentable native domain arithmetic.
An ignored input is corrected on the next reconciliation, even when the model has
not changed. The range handler shares the value-handler slot with numeric and
semantic handlers; a later handler replaces the earlier one. Single-value sliders
retain `on_value_changed(i32)` and cannot use a range handler. Typed construction
rejects that mismatch; strict erased validation rejects it before mutation.
Reserve inner padding around both thumbs at both domain endpoints.

## Textarea editing and authoritative model text

The new modifiers configure user editing:

```rust
use core::num::NonZeroU32;
use rs_lvgl_ui::prelude::*;

fn code_field(model: &str) -> View<()> {
    typed::textarea(model)
        .accepted_chars("0123456789")
        .input_max_length(NonZeroU32::new(4))
        .cursor_position(Some(2))
        .cursor_click_position(false)
        .into_view()
}
```

The example requires at least two Unicode scalar values in `model`.
`accepted_chars` takes a nonempty, NUL-free set of permitted input characters.
Omitting it on a later view restores unrestricted editing. The mounted adapter
retains the accepted-character CString for the entire native reference lifetime.
`input_max_length(Some(NonZeroU32))` limits edited character count;
`input_max_length(None)` and omission remove the limit. There is no zero-as-limit
API. Native text length and cursor positions count Unicode scalar values, not UTF-8
bytes or grapheme clusters. `cursor_position(Some(n))` requires an existing scalar
boundary, including the end; an out-of-range position rejects the view.
`None` leaves editing placement uncontrolled when model text is unchanged.
`cursor_click_position(false)` disables pointer cursor placement; omission restores
true. Model changes can still cause native cursor repositioning.

Editing filters do not truncate authoritative application text. Reconciliation
temporarily removes filters, assigns the full model string, restores filters and
applies an explicit cursor. A rejected native edit therefore restores the complete
model, including characters outside the accepted set or length limit. Feedback
suppression prevents this correction from generating another application action.
Font glyph coverage is independent from UTF-8 retention; test the actual font when
using combining marks or scripts outside its repertoire.

The public `WidgetKind::Textarea` payload now includes
`input: Option<Box<TextareaInputOptions>>`. `TextareaInputOptions` is opaque and
implements `Default`; ordinary applications configure it through modifiers.
Manual enum construction must supply the field. Editing settings allocate only
for configured textareas and do not add an options pointer to every other node.

## One chart descriptor

`chart(ChartSpec)` and `typed::chart(ChartSpec)` are the chart constructors.
`ChartSeries::new(id, color, data)` supplies stable series identity. X and Y axis
selection is independent through `.axes(ChartAxis, ChartAxis)`;
`ChartAxes` provides primary and secondary ranges for both coordinates.

```rust
extern crate alloc;
use alloc::vec;
use rs_lvgl_ui::prelude::*;

fn readings() -> View<()> {
    chart(ChartSpec::new(
        ChartType::Line,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        vec![ChartSeries::new(
            7,
            Color::rgb(24, 150, 200),
            ChartData::values([20, 45, 30]),
        )],
    ))
    .width(px(220))
    .height(px(100))
}
```

`ChartType` supports Line, Bar, Stacked, Curve and Scatter. Scatter requires XY
samples; the other forms require Y samples. Curve additionally requires the
effective provider's `LV_USE_VECTOR_GRAPHIC`. Series share a nonzero point count;
empty membership is valid. Duplicate series/cursor IDs, invalid ranges,
inconsistent lengths and incompatible sample kinds reject the entire render
before either native tree changes.

`ChartValue::Missing` represents a gap. `Value(i32::MAX)` is rejected because LVGL
reserves that integer for gaps. Bar requires every point to be present and a
history to be completely filled. Stacked rejects negative values, mixed Y-axis
selection and cumulative signed overflow. These checks cover the documented
native representation constraints; extreme sample/geometry combinations have
not undergone a complete native renderer overflow audit.

A compatible series prefix retains its native instances. Changes to series
membership, order or axis selection rebuild the affected suffix. Native arrays
contain copied samples; LVGL never borrows a temporary Rust vector. Native
allocation failure returns `UiError::Allocation`, retires the affected tree with
backing data still live, and permits a later render from canonical state.

`ChartCursor` has its own stable ID, color and horizontal/vertical/both direction.
Its target is a chart-local pixel position or a displayed point slot identified
by series ID and index. A missing sample hides a point cursor. Cursors update
with properties and layout/style/scroll changes; drawing callbacks do not mutate
chart geometry.

## Bounded history and recovery

`ChartHistory::new(epoch, NonZeroU32, ChartSampleKind)` holds application samples
in chronological order. `push_y` or `push_xy` appends one sample, evicting the oldest
at capacity. `snapshot()` copies the currently retained samples into one immutable
allocation; cloning that snapshot clones an Rc. View construction includes this
copying cost when it requests a new snapshot. There is no zero-copy or constant
whole-render cost guarantee. Compare full vectors and snapshots with the actual
batch size and capacity; larger batches can take longer through the append path
even when they write fewer native samples.

Supply snapshots with `ChartData::History(history.snapshot())`. Shift places the
newest samples at the right; unfilled leading slots are missing. Circular uses
sequence modulo capacity for displayed slots. `ChartUpdateMode` affects history;
explicit full vectors retain their supplied order.

The mounted adapter appends only when epoch, capacity, sample kind, sequence
metadata and actual retained overlap prove continuity. Repeating the same snapshot
writes no samples. Skipped renders with retained overlap can append a batch.
A gap, divergent clone, changed capacity/mode, replacement, remount or failed
application recovers by full synchronization. Mounted progress advances only
after successful application; constructing or discarding a view consumes nothing.

Use a distinct epoch for `clear(new_epoch)` or stream replacement. Clear with the
same epoch returns `EpochUnchanged`. Sequence exhaustion returns
`SequenceExhausted` before changing state; start a new epoch. Wrong-kind samples
and reserved values also fail without consuming sequence numbers. Capacity is
application configuration, not a library-imposed history limit. Native allocation
and index constraints are checked when the descriptor renders. History is in
memory; remount recovery does not imply reboot persistence.

With `update-metrics`, `UpdateMetrics` exposes label text writes, chart sample
appends, full sample writes and full synchronizations. One XY pair counts as one
sample. Counters exclude draw operations, allocator calls and uninstrumented
native setters. Counter reductions alone do not establish frame-rate gains.

## Custom properties, animation and deletion

`CustomWidget` has an associated `Props` and these hooks:

```rust
pub trait CustomWidget: 'static {
    type Props: 'static;
    fn validate(props: &Self::Props) -> Result<(), UiError>
        where Self: Sized { Ok(()) }
    fn create(&mut self, parent: &mut Obj<'_>, props: &Self::Props)
        -> Result<CustomObject, UiError>;
    fn apply_props(&mut self, object: &mut CustomObject, props: &Self::Props)
        -> Result<(), UiError>;
    fn tick(&mut self, object: &mut CustomObject, elapsed_ms: u32)
        -> Result<(), UiError> { Ok(()) }
    fn delete(&mut self, object: &mut CustomObject);
}
```

`validate` is pure preflight. It cannot inspect LVGL, mutate external state or use
a mounted instance. Both main and overlay descriptors validate before native
mutation. `create` returns one live direct child of its supplied parent through
unsafe `CustomObject::new`; a create error must clean up its partial construction
itself and receives no delete hook.

Mount calls `apply_props` after successful creation. Every reconciliation with the
same key and concrete widget type delivers new properties to the mounted instance,
including equal values, so it can correct native divergence. The unused incoming
widget instance drops; its boxed properties transfer to the mounted instance.
A different key or concrete widget type replaces the native subtree.

`Ui::tick(elapsed_ms)` calls only `tick`, without rebuilding views or applying
properties. A tick error stops that walk with the tree still mounted. All hooks
run on the UI thread; reentrant UI operations are rejected. Native callbacks must
not unwind and enqueue actions through explicit node handlers.

Properties are separately boxed at stable addresses. A successful `apply_props`
must retire native references to prior properties before returning. On partial
application failure, both prior and incoming properties survive until the affected
native subtree is deleted. The widget and backing data also survive native
LV_EVENT_DELETE callbacks, including after an initial apply or later sibling
failure. The delete hook runs exactly once for successful creation while the root
is live. It retires external registrations and must not delete the root, release
still-referenced backing early or reenter UI operations. Native deletion follows;
only then does Rust backing drop. A later valid render can recreate the tree.
`CustomObject::obj()` provides a hook-scoped borrowed Obj; raw pointers cannot
escape its lifetime contract.

The [portable depth recipe](../.agents/skills/rs-lvgl-ui/references/capability-depth.rs)
and [native Studio](component-studio.md) demonstrate property updates without
replacing the native object. The [validation guide](testing.md) separates native
readback, visible input, package execution and statistical host measurements.
