# UI API migration

## 0.8.0 migration from 0.7.0

Update `rs_lvgl_ui`, `rs_lvgl`, and any direct `rs_lvgl_sys`/`rs_lvgl_codegen`
dependencies to 0.8.0 together. The native provider remains vendored LVGL 9.6.

| Area | Action for 0.8.0 |
| --- | --- |
| Exhaustive `ImageSource` matches | Handle the added `Rgb888` variant. RGB888 static/shared sources and Canvas buffers validate native dimensions, stride and storage before mutation. |
| Target atomics | Provide native 32-bit atomic load/store for `tick_inc`, now also used without the `rust_timer` feature. |
| Tick source | Call `tick_inc` from one source. Its carried fractional millisecond uses separate atomic load/store, so concurrent writers can lose a fraction. Choose either incremental ticks or a custom clock callback. |
| Absolute uptime | `timer::update_clock` keeps `Result<(), TryFromIntError>` and now always returns `Ok(())`; it reduces elapsed milliseconds modulo 2^32. Track absolute uptime independently if it matters to the application. |
| Main loop | `task_handler()` remains valid. Use `timer_handler()` to obtain the next native timer deadline, and handle `NO_TIMER_READY` while still waking for input and newly readied work. |
| Large draw buffers | `DrawBuffer` allocates directly on the heap, including unoptimized builds; allocator ownership and native deletion obligations are preserved. |

`DisplayBuilder` adds explicit RGB888/RGB565/swapped byte buffers, PARTIAL/DIRECT/
FULL modes, single/double buffering, stride and invalidation granularity. Existing
`Display::register` remains synchronous RGB888 and `register_raw` follows the native
format. No fixed frame rate, automatic pacing, panel VRR or safe-area layout is
introduced. Choose periods from the device's measured budget.

See [configurable displays](display.md) for initialization, per-flush metadata,
unsafe asynchronous completion, retained image/Canvas storage and cleanup.

## 0.7.0 migration from 0.6.0

Version 0.7.0 introduces the following breaking changes. Update rs_lvgl_ui and
any direct rs_lvgl/rs_lvgl_sys dependencies to 0.7.0 together. Chart and custom
widgets expose one interface each; migrate callers to these signatures.

| 0.6.0 | 0.7.0 |
| --- | --- |
| `chart(chart_type, min, max, Vec<ChartSeries>)` | `chart(ChartSpec)`; also `typed::chart(ChartSpec)` |
| `ChartSeries { color, values }` | `ChartSeries::new(stable_id, color, ChartData::values(values))` in the spec |
| Chart min/max arguments | `ChartAxes` containing primary/secondary X/Y `ChartRange` values |
| `custom(widget)` | `custom(widget, props)` with `W::Props` |
| `CustomWidget::create(parent)` | `create(parent, &Props)` plus associated `type Props` |
| `CustomWidget::update(object, elapsed_ms)` | `apply_props(object, &Props)` for model input; `tick(object, elapsed_ms)` for time |
| Manual `WidgetKind::Textarea` literal | Include `input: Option<Box<TextareaInputOptions>>`; use modifiers to configure the opaque settings |

Migrate chart series with stable IDs that identify the same logical signal across
views. A basic Y chart uses `ChartType::Line`, `ChartData::values` and the previous
Y range; choose an explicit X range even when the presentation uses sample slots.
Scatter uses XY data. `ChartValue::Missing` is the explicit native gap.
History snapshots recover discontinuities from canonical application data.

Implement custom pure validation and property application, even if Props is `()`.
Move elapsed-time animation into `tick`; replace shared-state/key-change refresh
workarounds with explicit Props. Equal properties are applied to the mounted
instance; changing the concrete type or key replaces it. Deletion retains widget
and property backing until native callbacks finish. There are no chart/custom
aliases, alternate constructor names or adapters for the old signatures.

See [controls, chart history and custom properties](ui-capability-depth.md) for
compilable construction examples, representation limits and failure behavior.
Default features and the native provider remain unchanged.

## Released 0.6.0 migration from 0.5.0

Version 0.6.0 introduces breaking UI changes relative to 0.5.0. Update the
dependency and migrate the affected names, units, layouts and widget parts:

```toml
[dependencies]
rs_lvgl_ui = { version = "0.6.0", default-features = false, features = ["alloc", "widget-slider"] }
```

The renamed functions/types/methods have no compatibility aliases or deprecated
forwards. Low-level `rs_lvgl` and raw C names retain their native API.

## Names and exports

Ordinary constructors, `typed::` constructors, root exports and prelude exports
use the same names. Widget feature names such as `widget-bar`, `widget-msgbox`
and `widget-animimage` are unchanged; importing a widget does not enable it.

| Released 0.5.0 | 0.6.0 |
| --- | --- |
| `obj()` / `typed::obj()` | `container()` / `typed::container()` |
| `bar(...)` / `typed::bar(...)` | `progress_bar(...)` / `typed::progress_bar(...)` |
| `msgbox(...)` / `typed::msgbox(...)` | `message_box(...)` / `typed::message_box(...)`; still inline, no implicit modal behavior |
| `anim_image(...)` / `typed::anim_image(...)` | `animated_image(...)` / `typed::animated_image(...)` |
| `typed::kind::Obj/Bar/Msgbox/AnimImage` | `Container/ProgressBar/MessageBox/AnimatedImage` |
| `WidgetKind::Obj/Bar/Msgbox/AnimImage` | `Container/ProgressBar/MessageBox/AnimatedImage`; same diagnostic WidgetType variants |
| `ScrollDir`, `scroll_dir(...)` | `ScrollDirection`, `scroll_direction(...)` |
| `ScrollDir::HOR/VER` | `ScrollDirection::HORIZONTAL/VERTICAL`; bitwise combinations preserved |
| `Tile::dir` | `Tile::directions`; `Tile::new` accepts ScrollDirection |
| `Align::TopMid/BottomMid/LeftMid/RightMid` | `TopCenter/BottomCenter/CenterLeft/CenterRight` |
| `LabelLongMode` | `TextOverflow`; configure labels with `.text_overflow(...)` |
| `Dots`, `ScrollCircular` | `Ellipsis`, `ScrollLoop`; Wrap/Scroll/Clip preserved |
| `WidgetKind::Label::long_mode` | `text_overflow` field |
| `msgbox_title`, `msgbox_buttons` | `message_box_title`, `message_box_buttons` |
| `spinner_anim` | `spinner_animation`; duration is milliseconds and sweep is whole degrees |
| `anim_duration`, `anim_duration_value` | `animation_duration`, `animation_duration_value`; milliseconds |
| `ANIM_REPEAT_INFINITE`, `ANIM_IMAGE_MAX_FRAMES` | `ANIMATION_REPEAT_INFINITE`, `ANIMATED_IMAGE_MAX_FRAMES` |
| `Style::neutral()` | `Style::plain()`; same explicit base values |
| `bg_grad_color`, `bg_grad_dir` | `background_gradient_color`, `background_gradient_direction` |
| `bg_main_stop`, `bg_grad_stop` | `background_gradient_start`, `background_gradient_end`; positions 0..=255 |
| `bg_main_opacity`, `bg_grad_opacity` | `background_gradient_start_opacity`, `background_gradient_end_opacity` |
| `text_letter_space`, `text_line_space` | `letter_spacing`, `line_spacing`; signed pixel spacing |
| `outline_pad` | `outline_offset`; signed pixels |
| `image_recolor`, `image_recolor_opacity` | `image_tint`, `image_tint_opacity` |
| `TransitionProperty::GradientColor` | `BackgroundGradientColor` |
| `TransitionProperty::ImageRecolor/ImageRecolorOpacity` | `ImageTint/ImageTintOpacity` |
| `TransitionProperty::OutlinePad/LetterSpace/LineSpace` | `OutlineOffset/LetterSpacing/LineSpacing` |
| `Property::SpinnerAnim/MsgboxTitle/MsgboxButtons` | `SpinnerAnimation/MessageBoxTitle/MessageBoxButtons` |

`CustomObject::obj()`, `rs_lvgl_ui::Obj` and unsafe raw/native extension contracts
still expose native handles. They do not construct ordinary UI containers.

## Automatic plain styles and reusable patches

Container/row/column/grid/label constructors already apply plain base values, as
do spans and arc labels where enabled. Write local fields directly:

```rust
use rs_lvgl_ui::prelude::*;

fn heading(theme: &Theme) -> View<()> {
    column([label("Fan settings").font(theme.title_font).text_color(theme.text)])
        .width(percent(100)).padding(16).gap(12)
}
```

Remove a repeated `style(Style::plain())` only when it does not intentionally
reset a previous write. For example, `.padding(12).style(Style::plain())` resets
padding to zero; deleting that call changes behavior. A plain patch on a native
button can intentionally remove its surface. Those resets remain explicit.

`Style::plain()` sets transparent background, zero border/radius/padding/gap,
content sizing and top-left placement among its base values. It leaves text
inheritance, extended styles, offsets, floating and flex placement unset. It is
neither an empty patch nor a complete reset. `style()` and `Style::default()` set
nothing. Main `.style(patch)` and flat modifiers merge explicit fields in call
order, including explicit zero and false. Theme presets read current token
values; cached patches do not subscribe to theme changes.

## Transform and image units

Scale and angle inputs now require explicit values in ordinary/typed image
constructors, WidgetKind image descriptions, Style/Node transforms and animations.

| Old input | Development input | Native result |
| --- | --- | --- |
| `transform_scale_x(128)` | `transform_scale_x(Scale::percent(50))` | 128 |
| `transform_scale_y(384)` | `transform_scale_y(Scale::percent(150))` | 384 |
| `image_scaled(source, 256)` | `image_scaled(source, Scale::percent(100))` | 256 |
| `image_resource_scaled(resource, 192)` | `image_resource_scaled(resource, Scale::percent(75))` | 192 |
| Arbitrary old scale `s` | `Scale::ratio(s as u32, 256)?` | Exactly s for valid u16 s |
| `transform_rotation(900)` | `transform_rotation(Angle::degrees(90))` | 900 |
| `transform_rotation(455)` | `transform_rotation(Angle::tenths(455))` | 455 |
| `Animation::new(AnimationProperty::Rotation, 0, 900, 200)` | `Animation::rotation(Angle::degrees(0), Angle::degrees(90), 200)` | Same endpoints |
| Raw ScaleX/ScaleY animation | `Animation::scale_x/y(Scale, Scale, duration_ms)` | Typed endpoints |

`Scale::percent(u16)` rounds to the nearest native 1/256, ties upward.
`Scale::ratio(u32, u32)` uses the same rounding and returns InvalidProperty for a
zero denominator or a rounded result above 65535. Even a small positive ratio
may round to zero. Style/animation scales accept zero; image scales reject it.
Unrepresentable percentages fail tree validation before native mutation.

`Angle::degrees(i16)` multiplies in i32; `Angle::tenths(i32)` preserves fractional
precision. Native rotation accepts -32768..=32767 tenths (-3276.8..=3276.7 degrees).
Values outside that range reject before mutation; no wrap, modulo or clamp is
applied. Rejected updates preserve the valid tree and running animation.
Scalar AnimationProperty keeps X/Y/Width/Height/Opacity. TransitionProperty can
still select Rotation/ScaleX/ScaleY because it carries no untyped endpoints.
Spinner sweep uses whole degrees and keeps its separate u32 control contract.

Proportional scaling sets both `transform_scale_x` and `transform_scale_y` to
the same `Scale`; setting one axis stretches the original shape. A transform
changes drawing without enlarging its layout allocation. Reserve a containing
slot for the rotated bounds and set a centered pivot. The runnable clarity
recipe uses a 24 px square, two 150% scales, a (12, 12) pivot and a 64 px slot.

## Independent flex placement and floating

The old `flex_align(main, cross)` wrote native main placement once and used
cross for both item placement and line/track placement. Preserve its positions
with all three new writes:

```rust
use rs_lvgl_ui::prelude::*;

fn centered_row() -> View<()> {
    row([label("Name"), button("Save")])
        .justify_content(FlexAlign::SpaceBetween)
        .align_items(ItemAlign::Center)
        .align_content(FlexAlign::Center)
}
```

`justify_content` maps main placement. `align_items` maps placement within a
line; ItemAlign accepts Start/Center/End only. `align_content` maps native track
placement, including a single line. Both FlexAlign properties additionally accept
SpaceEvenly/SpaceAround/SpaceBetween. Each field merges and clears independently;
unset restores Start. No CSS stretch behavior is implied. Version 0.7.0 adds explicit
`flex_wrap(bool)` and `flex_reverse(bool)` controls. If an old
cross value used a distribution variant, LVGL treated item placement as Start;
use ItemAlign::Start and retain that FlexAlign value for align_content.

`floating(true)` retains both layout exclusion and parent-scroll exclusion.
Align/offset position the floating child relative to its parent. It stays anchored
as siblings scroll. It is not renamed to absolute and does not gain CSS semantics.

## Semantic and native parts

| Widget | `thumb_style` / `thumb_state_style` | `progress_style` / `progress_state_style` |
| --- | --- | --- |
| Slider | Native Knob | Native Indicator, filled region |
| Switch | Native Knob | Unsupported |
| ProgressBar | Unsupported | Native Indicator, filled region |
| Other widgets | Unsupported | Unsupported |

Typed misuse fails the Supports capability bound. Erased misuse is ignored by
default, emits its structured property when diagnostics is enabled, and rejects
before mutation under strict-properties. Text-overflow uses the Label capability
with the same erased diagnostic policy.

Native selectors remain available as `NativePart`, `native_part_style` and
`native_part_state_style`. For example, an Arc indicator keeps its arc-specific
meaning; it is not automatically called progress. `state_style` targets Main.

Part/state lists preserve their existing replacement order: the last patch for
a given part/state replaces the entire earlier patch, including omitted fields.
Combine related fields in one patch. Semantic and native calls share the same
list/order. Removing a patch restores base/class style on the existing object;
states switch natively without requiring an application render.

## Host memory measurements

Compared with the released 0.5.0 source, the 0.6.0 descriptors measured
on macOS ARM64 with Rust 1.99.1 in the unoptimized test profile have these sizes:

| Value | Released 0.5.0 | 0.6.0 |
| --- | --- | --- |
| `Style` | 216 bytes | 232 bytes |
| `View<()>` | 632 bytes | 648 bytes |

In the equivalent form fixture, each of the erased/typed/typed-binding variants
still performs 160 Rust allocations across 32 constructions. Cumulative requested
bytes increase from 82,464 to 84,512 and peak extra bytes from 2,577 to 2,641.
Native heap baselines remain unchanged in the measured update workloads; three
consecutive lifecycle runs balance both Rust and native heaps. These results
describe these host fixtures, not target-device RAM, fragmentation or firmware
size. Use the cost and lifecycle entrypoints in [host validation](testing.md)
when assessing a target application's budget.

## Validation

The native `api_clarity` tests cover actual unit conversion/ranges, rejected
updates, animation endpoints, independent flex coordinates, floating scroll
behavior, selector states/order/removal and erased diagnostics. External
consumers compile root/prelude/typed positives and deliberate old-path/type
failures. [Host validation](testing.md) lists the broader feature, heap, native
example and skill checks. Registry consumers are verified in the release workflow; device acceptance
requires the target application and hardware.
