# View styling and application actions

The `rs_lvgl_ui` 0.5.0 API uses flat style modifiers, reusable patches and
component-specific action types.

## Local properties and reusable patches

View constructors prepare their own embedded style description. Set local
properties directly on the view:

```rust
use rs_lvgl_ui::prelude::*;

fn heading<M>(theme: &Theme) -> View<M> {
    label("Fan settings")
        .font(theme.title_font)
        .text_color(theme.text)
}

fn settings_layout(theme: &Theme) -> View<()> {
    column([
        heading(theme),
        button("Save")
            .background(theme.accent)
            .text_color(theme.background)
            .padding(8)
            .radius(8)
            .shadow_width(0)
            .state_style(State::PRESSED, style().opacity(200)),
    ])
    .width(percent(100))
    .height(content())
    .gap(12)
    .padding(16)
}
```

`style()` creates an unset reusable patch. Use it for shared values and
part/state selectors. `Style::surface(theme)`, `Style::accent(theme)` and other
presets also return reusable values. A preset may be combined with flat
overrides: `obj().style(Style::surface(theme)).padding(20)`.

`Style::default()` remains a Rust `Default` construction primitive. Application
code does not need it to initialize a view. `Style::neutral()` has a different
purpose: it explicitly clears non-inheritable container appearance/layout
defaults, while leaving inheritable text properties unset. Applying it to a
button can deliberately remove that button's native surface.

## Units, explicit fields and precedence

| Input | Meaning |
| --- | --- |
| `px(120)` | `Size::Px(120)`, a pixel dimension |
| `percent(100)` | `Size::Percent(100)`, relative to the parent; values above 100 are retained |
| `content()` | `Size::Content`, size from native content/layout |
| `.padding(12)`, `.gap(8)` | Pixel spacing, as in the existing native contract |
| `.grow(1)` | Proportional flex growth, not a pixel or percentage dimension |

Width, height and all four min/max constraints take `Size`. `.width(100)` is a
compile error. The helpers preserve the existing representation; they do not
infer units, clamp values or allocate. Negative pixel dimensions are rejected
by tree validation before native mutation. Signed offsets retain their existing
meaning. `gap` affects the main axis of a row or column and both axes of a grid;
it does not add layout behavior to a plain object.

Flat modifiers and `.style(patch)` write the same embedded style. The last
explicit write to each field wins. For example:

```rust
use rs_lvgl_ui::prelude::*;

let shared = style().padding(12).gap(8);
let view: View<()> = column([])
    .style(shared)
    .padding_left(0)
    .style(style().padding_top(4));
```

The left padding is 0, the top is 4, the right/bottom are 12, and the gap is 8.
An unset field does not overwrite an earlier value. An explicit `0` or `false`
does. Text color/font remain inheritable when omitted; a child does not get
implicit black text from the unset patch's stored placeholder values.

`background(color)` writes both color and opacity 255.
`background_opacity(128)` writes opacity alone. Put the latter after the color
to request a translucent fill. `.background_opacity(128).background(color)`
ends with an opaque fill, following the same call-order rule.

Flat modifiers target the main style. `part_style`, `state_style` and
`part_state_style` continue to take explicit `Style` patches. Native states
switch without an application render. Removing a selector patch during an
update restores its base/class values on the existing object. Object-wide
fields retain their main-style restrictions. A retained `font_resource`
continues to override a static main-style font until removed.

## Themes and component actions

`.font(theme.title_font).text_color(theme.text)` reads two values from the
current theme. `Style::title(theme)` packages those values; it does not create
a subscription. Build theme-dependent views/patches inside `App::view` or a
helper it calls. `Ui::set_theme` marks the UI dirty, and the next `mount`,
`update` or `drain` evaluates the view with the new theme. A previously cached
`Style` remains the value it was when constructed. Application theme tokens
do not switch LVGL's native theme.

`View<M>` describes which type its handlers produce. `M` is a Rust generic
parameter; each application chooses its action type. `App::Action` is the trait's
associated type name, so a fan screen writes `type Action = FanSettingsAction` and
returns `View<FanSettingsAction>`. The enum's variants describe inputs such as
`SpeedChanged(i32)` and `Save`; component fields hold the actual speed/saved
state, and `update` implements the operations. Named action parameters use
`action`, or a more specific name such as `navigation_action`; an intentionally
unused argument may use `_`.

### Application action type

`App::Action` connects an application's action type to the runtime:

| Context | Declaration |
| --- | --- |
| Application implementation | `type Action = FanSettingsAction;` |
| Generic action type | `Self::Action` or `<Settings as App>::Action` |
| Generic application bound | `A: App<Action = FanSettingsAction>` |
| Action handler | `fn update(&mut self, action: FanSettingsAction)` |
| View output | `View<FanSettingsAction>` |

`Action` is the trait member name. Applications choose the concrete type on the
right, such as `FanSettingsAction` or `CounterAction`. The update parameter is a
local binding, so `fn update(&mut self, action: FanSettingsAction)` does not
require the concrete enum to be named `Action`. A different component can use
`CounterAction` or another descriptive application type.

## Reading style values

Style setters return the modified value for chaining. The corresponding getters
use a `_value` suffix:

| Property | Getter |
| --- | --- |
| Background color | `.background_value()` |
| Background opacity | `.background_opacity_value()` |
| Text color | `.text_color_value()` |
| Padding edges | `.padding_left/right/top/bottom_value()` |
| Flex growth | `.grow_value()` |

## Memory and validation boundaries

Flat calls update the `Style` already stored inside `Node`; they add no node
fields, per-property heap objects or native calls during description
construction. Reusable patches are plain `Copy` values. Part/state lists,
children, owned text and runtime bookkeeping can still allocate. LVGL allocates
its own native property storage when the description is mounted/applied.
An unchanged mounted style continues to skip native reapplication.

The host cost fixture records descriptor sizes, cumulative Rust allocation
calls/requested bytes, peak/live Rust bytes and LVGL live bytes/blocks plus
high-water usage. It measures idle, unchanged, style-change, keyed reorder and
theme-change workloads after declared priming. It does not count native
allocation calls or establish board RAM, timing or linked firmware size.
Full lifecycle checks separately verify both heaps with deliberate leak
controls. See [testing](testing.md) for commands and
[runtime/resource contracts](ui-runtime.md) for lifetime obligations.
