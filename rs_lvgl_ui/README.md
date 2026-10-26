# rs_lvgl_ui

The [browser studio](../website/docs/en/studio.mdx) runs all nine component categories in WebAssembly. It shares these Rust views with the desktop studio; see [build instructions](../website/README.md).


A declarative, typed Rust UI layer for LVGL 9.6, built on the `rs_lvgl` bindings in
this workspace.

The
[repository README](https://github.com/umbrella22/rs_lvgl/blob/main/README.md) covers native dependencies and simulator setup.

Application code describes a screen as a `View<Msg>` tree, updates one state
value through messages, and lets the layer create, update, reorder and delete
the native LVGL objects. LVGL remains the renderer, layout engine and animation
engine.

```text
platform event -> C callback -> EventQueue -> Ui::drain -> App::update
  -> App::view -> View tree -> reconciler -> LVGL objects
```

## Design rules

- One UI thread has exclusive access to LVGL.
- C event callbacks only enqueue messages; they never mutate the tree.
- `App::update` changes state; `App::view` is pure and returns a descriptor tree.
- The reconciler is the only code that creates, updates or deletes LVGL objects.
- `rs_lvgl_ui` never registers displays, input devices, timers or RTOS tasks.
- The crate is `no_std` and requires only `alloc`.

## Features

| Feature | Default | Meaning |
| --- | --- | --- |
| `alloc` | yes | Enables allocator support in the `rs_lvgl` dependency. Required on bare metal. |
| `simulator` | no | Host-only: enables LVGL's native SDL2 backend for the gallery. No Rust `sdl2` bindings are used. |
| `input-keypad` | no | Typed keypad driver, explicit per-UI focus groups and modal focus scopes. |
| `input-gridnav` | no | Spatial arrow navigation; includes `input-keypad`. |
| `capability-gallery` | no | Host demo feature set: simulator, grid navigation, Canvas, media, IME and CJK 16. |
| `leak-check` | no | Isolated host lifecycle harness; run through `scripts/check-ui-leaks.py` with its BUILTIN allocator configuration. |
| `no-autoinit` | no | Forwards `lvgl/unsafe_no_autoinit`; target platforms call `rs_lvgl::init()` themselves. |
| `components` | yes | The `rs_lvgl_ui::components` module. Uses only the core widgets. |
| `widgets-all` | yes | The original widget set, including typed Canvas and raw list/menu/win; optional media/IME extensions are separate. |
| `widget-<name>` | original set via `widgets-all` | One widget and its typed constructor; QR/barcode/GIF/Lottie/Pinyin are opt-in. |
| `font-cjk-14`, `font-cjk-16` | no | Vendored Source Han Sans SC CJK fonts. |
| `fonts-vendored` | yes | Montserrat 12, 16, 28 and 48. |
| `font-montserrat-<size>` | via `fonts-vendored` | One Montserrat size (8–48, even sizes) and `Font::montserrat_<size>`. |

Target builds should use `--no-default-features --features alloc,no-autoinit`
plus the widgets and fonts they use (see [Choosing widgets](#choosing-widgets)).

## Choosing widgets

LVGL's linker sections alone cannot drop unused widgets: the reconciler and
LVGL's default theme reference every enabled widget class. `rs_lvgl_ui` therefore
turns each optional widget into a Cargo feature, and `rs_lvgl_sys` writes a
`lv_conf.h` with `LV_USE_<WIDGET>` set only for the enabled ones, so disabled
widgets are not compiled at all. The defaults keep the full set.

```toml
[dependencies]
[dependencies.rs_lvgl_ui]
version = "0.2"
default-features = false
features = [
    "alloc", "components", "widget-slider", "widget-switch", "font-montserrat-20",
]
```

- Always available: `obj`, `row`, `column`, `grid`, `label`, `button`, custom
  widgets, `raw` and `Font::montserrat_14`. Composite components require
  the `components` feature.
- Optional widgets: `animimage`, `arc`, `arclabel`, `bar`, `buttonmatrix`,
  `calendar`, `canvas`, `chart`, `checkbox`, `dropdown`, `image`, `imagebutton`,
  `keyboard`, `led`, `line`, `msgbox`, `roller`, `scale`, `slider`, `span`,
  `spinbox`, `spinner`, `switch`, `table`, `tabview`, `textarea`, `tileview`.
  Each `widget-<name>` enables its ordinary constructors in the crate root and
  `prelude`, their
  `WidgetKind` variant and the widget-specific modifiers (for example
  `View::placeholder` needs `widget-textarea`).
- LVGL dependencies are enabled automatically: `keyboard` brings in
  `buttonmatrix` and `textarea`, `calendar` brings in `buttonmatrix`, `slider`
  brings in `bar`, `spinbox` brings in `textarea`, `spinner` brings in `arc`, and
  `animimage`, `msgbox`, `canvas` and the raw `menu` and `win` bring in `image`.
  Dependencies are compiled in C but stay out of the typed API unless their own
  feature is enabled. The calendar header dropdown follows `widget-dropdown`.
- `Theme::title_font` uses Montserrat 28 when `font-montserrat-28` is enabled
  and falls back to Montserrat 14 otherwise.
- LVGL widgets that have no typed node (list, menu, win) are enabled
  through `rs_lvgl_sys/widget-<name>` when `widgets-all` is off.
- Cargo unifies features across the dependency graph: any crate that enables
  a widget, or keeps `rs_lvgl_ui`'s default features, turns it on for the whole
  build. Check `cargo tree -e features -i rs_lvgl_sys` when a widget is not being
  removed.
- LVGL's official widgets demo (`rs_lvgl_sys/lv-demo-widgets`) needs the full
  configuration and disables this selection with a build warning.

Builds with only some widgets allow `dead_code` and related lints for helpers
that are shared by several widgets; the full set keeps every lint.

## Quick start

```rust
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum Msg {
    SpeedChanged(i32),
}

struct Settings {
    speed: i32,
}

impl App for Settings {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::SpeedChanged(value) => self.speed = value,
        }
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        column([
            label("Fan").style(Style::title(theme)),
            slider(0, 100, self.speed)
                .key(Key::Named("speed"))
                .on_value_changed(Msg::SpeedChanged),
        ])
        .style(Style::default().gap(12))
    }
}
```

The platform creates the display, mounts the UI and pumps it:

```rust
rs_lvgl::init();
let display = rs_lvgl::Display::register::<_, { 320 * 240 }>(320, 240, |_| {})?;
let mut ui = Ui::new(&display, Theme::dark())?;
let mut app = Settings { speed: 40 };
ui.mount(&app)?;

loop {
    rs_lvgl::tick_inc(tick);
    ui.timer_handler();
    ui.drain(&mut app, 32)?;
}
```

`drain` processes at most the requested number of messages, then calls `view`
once and reconciles the result. A drain that processed no message returns
without calling `view` (`DrainStats::reconciled == false`) unless
`Ui::mark_dirty` or `Ui::set_theme` requested a render, so it is cheap to call
on every frame. State that changes outside messages (a clock or sensor read in
`view`) must call `Ui::mark_dirty()` or `Ui::update(&app)`. `Ui::tick(elapsed_ms)` advances custom-widget
animation; call it from the render loop, outside `lv_timer_handler`.

## Child order and composition

`View<M>` is a description of a UI subtree. `Node<M>` is the backing type of
that alias; ordinary application and component functions return `View<M>`.
Constructing a view does not create native LVGL objects.

`column(children)` arranges children in the normal flex flow from top to bottom
in iteration order. It accepts arrays, vectors and iterators. Child sizes,
spacing and alignment determine the pixel positions. `row` also preserves
iteration order; `grid` uses each child's `.grid_cell(...)` for placement.

Named variables make a screen's sections visible, and ordinary functions share
structure or styling:

```rust
use rs_lvgl_ui::prelude::*;

fn page_title<M>(text: &str, theme: &Theme) -> View<M> {
    label(text).style(Style::title(theme))
}

fn settings_view(theme: &Theme) -> View<()> {
    let content = label("Notification preferences");
    let title = page_title("Settings", theme);
    column([title, content]).style(Style::default().gap(12))
}
```

The title appears before the content because it comes first in the child
collection, even though it is constructed later. A helper returns a description
within the same view tree; it does not add independent state or a render cache.
Each parent takes its child values by move. Call the helper again to create a
view for another location.

Child order and identity are separate. Stable `Key::Named` or `Key::Id` values
identify dynamic children across inserts, removals and reorders. An array index
used as a key follows the position, so it does not identify a reordered item.

## Semantics

- `Key::Auto` matches by position among `Auto` siblings. `Named` and `Id` match
  by value and preserve native object identity across inserts, removals and
  reorders. Keys must be unique among siblings. A key change on a node that is
  updated in place (the root or a tab page) replaces its native subtree, like a
  kind change.
- A whole `Style` value is compared. `Style::default()` sets nothing and
  inherits the LVGL widget class default, so sliders keep their track and
  buttons keep their surface. `Style::neutral()` explicitly clears every field
  and is the base for plain containers and labels, except for the inherited
  text properties (text color, font, text alignment) and arc properties, which
  stay unset so a label inherits the text color and font of its container.
  Plain containers and the `Ui` root also drop LVGL's default-theme card style,
  so they start transparent and borderless. `.style(...)` merges the fields the
  supplied value sets over the node's kind base.
- `bg(color)` sets the color and an opaque fill; `bg_opacity(opa)` sets only
  the opacity. `Style::surface(theme).merged(Style::default().bg_opacity(128))`
  keeps the surface color. Call `bg_opacity` after `bg` on the same value.
- `Theme` supplies application color/font tokens; it does not switch LVGL's
  native theme. `Style::accent(theme)` and the composite action buttons use
  flat surfaces with `shadow_width(0)` so a native light-theme shadow does not
  appear as a gray lower edge on dark backgrounds. Plain `button` and
  `Style::default()` retain native defaults. For a custom flat button, set
  `shadow_width(0)`; for elevation, set the shadow width, color, opacity and
  offset explicitly after the preset.
- Messages must implement `Clone`; a click handler stores its message and clones
  it for each click. `on_text_changed` receives a borrowed `&str`; copy it if the
  message must retain the text.
- Handlers are independent: `on_click`, `on_value_changed`, `on_text_changed`
  and `on_event` can be combined on one node; repeating one builder replaces
  only that handler. Native callbacks are registered lazily and only for the
  event codes in use (`LV_EVENT_ALL` with `on_event` or input payload handlers);
  a node without handlers has no callbacks. Direct handlers see the node's own
  events. `on_bubbled_event` explicitly observes descendant events.
- Handlers do not change LVGL flags. Use `View::clickable(bool)` and
  `View::scrollable(bool)` to override the class defaults.
- Interactive values are controlled: slider, bar, arc, switch, checkbox,
  dropdown, roller, spinbox, textarea text and the active tab are compared
  with the native widget on every render, so the native widget always shows
  the value from `view`. A user change that the application ignores or clamps
  is reverted on the next render. Programmatic writes suppress the next native value event,
  preventing feedback loops.
- `grid(columns, rows, children)` takes `GridTrack` columns/rows and its child
  collection; children use `grid_cell` and
  `grid_align`. `tabview` takes one child per tab page and reports the
  active index through `on_value_changed`.
- `View::part_style(Part, Style)` styles a specific widget part (indicator,
  knob, scrollbar, selected, items, cursor) and restores class defaults when the
  part style is removed.
- `View::state_style(State, Style)` and `View::part_state_style(Part, State,
  Style)` style a widget while it is in a state (`State::PRESSED`,
  `CHECKED`, `FOCUSED`, `FOCUS_KEY`, `EDITED`, `HOVERED`, `DISABLED`, combined
  with `|`). LVGL switches between them natively; no message or render is
  involved. Object-wide fields (`hidden`, `disabled`, `gap`, `floating`,
  `flex_align`) only apply in the main style.
- `View::checked(bool)` controls `LV_STATE_CHECKED` on any widget, so a list
  row or a tab-like button can show `State::CHECKED` styles for the selected
  item. Switches and checkboxes keep their own checked value and ignore it.
- `View::map(f)` converts every message of a subtree. Components with their
  own message type are embedded with `component::view(&state).map(Msg::Item)`.
  Object identity, keys and handlers are kept; `f` should be a pure conversion.
- `Style::floating(true)` takes a widget out of its parent's layout and
  scrolling; `Style::align` and `Style::offset(x, y)` then place it relative
  to the parent. Use it for badges, floating action buttons and backdrops.
- `Style::flex_align(main, cross)` places the children of a `row` or
  `column` with `FlexAlign` (`Start`, `Center`, `End`, `SpaceEvenly`,
  `SpaceAround`, `SpaceBetween`), like CSS `justify-content` and
  `align-items`. `cross` also sets LVGL's track placement, so it moves the
  items of a single-line container. Other kinds ignore it; unset restores
  `Start`.
- `App::overlay` (default `None`) returns a second tree mounted on the
  display's top layer, above the view. The overlay root fills the display and
  is not clickable, so input reaches the view except where an overlay widget is
  hit; add a full-size clickable backdrop to make an overlay modal. Returning
  `None` deletes the overlay tree. Both trees share the message queue, the
  `tick` walk and `stats`; keyboard ids resolve across both trees and should
  be unique across them.
- `rs_lvgl_ui::symbol` contains LVGL's built-in icon glyphs (`symbol::OK`,
  `symbol::CLOSE`, `symbol::LEFT`, ...) for label and button text. The default
  Montserrat fonts include them; custom fonts must embed them to render them.
- `keyboard(target)` binds to the textarea registered with
  `keyboard_id(target)`. `keyboard_id` values must be unique across the whole
  tree. A binding is applied only when its target changes, and a keyboard is
  unbound before its textarea is deleted. Textarea focus and keyboard Ready/Cancel events are
  delivered through `on_event`, so the application decides when to show or hide
  the keyboard.
- `ImageSource::rgb565(width, height, &'static [u8])` builds a safe static image
  descriptor; the mounted tree keeps it alive. `ImageSource::Text` and the unsafe
  `ImageSource::from_raw` descriptor path remain available; the raw variant
  wraps an opaque `RawImage`, so it can only be built through `from_raw`.
- Widgets: `obj`, `row`, `column`, `grid`, `tabview`, `label`, `button`,
  `slider`, `bar`, `switch`, `image`, `checkbox`, `textarea`, `dropdown`,
  `arc`, `scale`, `chart`, `keyboard`, `roller`, `spinbox`, `spinner`, `led`,
  `line`, `button_matrix`, `table`, `tileview`, `msgbox`, `calendar`, `spans`,
  `image_button`, `anim_image` and `arc_label`, plus `raw`/`custom` escape
  hatches. `WidgetKind` is `#[non_exhaustive]`.
- LVGL 9.6 deprecates `lv_list`, `lv_win` and `lv_menu` ("build ... from base
  widgets instead"), so they are not wrapped; build them from `column`, `row`,
  `button` and `label`. `canvas` retains an aligned native drawing buffer;
  custom widgets remain available for other rendering pipelines.
- `on_value_changed` values: roller → selected index (the real index in
  `RollerMode::Infinite`); spinbox → integer value; button matrix → pressed
  button index counted row by row, or `-1`; table → the selected cell packed
  as `(row << 16) | column`, decoded with `rs_lvgl_ui::table_cell`; tileview →
  active tile index (children order), reported when a swipe settles; calendar
  → the pressed date packed as `year * 10000 + month * 100 + day`, decoded
  with `CalendarDate::from_value`, or `-1` for day names and adjacent-month
  days; message box → clicked footer button index.
- `tileview(tiles, active)` takes one child per `Tile` (column, row and
  allowed `ScrollDir`); `active` is controlled. Changing the tile layout
  recreates the tileview.
- `msgbox(text)` is always inline (a child of its parent, never a modal
  on the top layer) and has no close button, because LVGL's close helper
  deletes the object behind the mounted tree. Hide it from the view instead.
  Children are placed below the text. Adding or removing the title, or
  changing the number of footer buttons, recreates the message box; texts are
  patched in place.
- `calendar(year, month)` shows a month without header controls; set
  `today` and `highlighted` dates explicitly. Events of the calendar's inner
  button matrix are reported by the calendar node.
- `spans` reuses native spans when the count is unchanged; a span's
  `color`/`font` of `None` inherits from the group.
- `image_button` and `anim_image` keep generated RGB565 descriptors
  and the frame pointer array alive for LVGL; animated images accept
  `1..=ANIM_IMAGE_MAX_FRAMES` frames and loop until `repeat` is set.
- `arc_label(text, radius, start_angle, angle_size)` draws text along an
  arc; `angle_size` must be within `0..=360`.
- `line` and `button_matrix` own the point buffer and the button
  map that LVGL reads without copying; a changed value installs the new buffer
  before the old one is freed.
- `Style` also supports per-side padding, text alignment, min/max sizes,
  `hidden`/`disabled` states, arc indicator color/width, `anim_duration`,
  `offset`, `floating` and `flex_align`.
- `Ui` is `!Send` and `!Sync`. Safe LVGL handles never escape the crate.

## Components

The default `components` feature adds `rs_lvgl_ui::components`, composite
components built only from plain nodes. They replace LVGL widgets that are
deprecated in v9.6 and removed in v10 (`lv_list`, `lv_win`, `lv_menu`) and add
overlay building blocks:

| Module | Kind | Purpose |
| --- | --- | --- |
| `list` | stateless | `list`, `section` and `ListItem` rows with icon, text, detail or trailing node, chevron, pressed and selected styles |
| `panel` | stateless | `Panel`: title, header actions (`icon_button`) and a content area |
| `nav` | stateful | `NavState<P>` page stack, `NavMsg<P>`, `link` rows and a `view` with a back button |
| `file_explorer` | application-supplied entries | `FileEntry`, `view`; identities/messages separate from display names |
| `dialog` | stateless overlay | `Dialog`: modal card over a dimming backdrop, footer buttons, `on_dismiss` |
| `toast` | stateful overlay | `Toasts` queue with per-toast duration, `advance(elapsed_ms)` and a `view` |

Components follow the application's Elm-style split. Stateless builders
return `View<M>` that emit the application's own messages. Stateful
components keep a plain state struct in the application; `update` changes it
and `view` only reads it. Visibility is state too: a dialog is shown while
`App::overlay` returns it, and closing it is a message. Toasts expire when the
application calls `Toasts::advance`, for example from a tick message the
platform posts while toasts are visible. Combine several overlays with
`components::layers`. A component with its own message type embeds with
`View::map`.

See `examples/components.rs` for a settings app that uses all of them.

## Touch, keys and scrolling

Input payload handlers are available with every widget selection. They preserve
the existing payload-free `Event` enum and compose with the click/value/text
handlers and `View::map`:

| Handler | Message input |
| --- | --- |
| `on_key` | Full `u32` LVGL key code; NEXT/PREV are consumed by group navigation |
| `on_rotary` | Signed `i32` rotary delta |
| `on_gesture` | `GestureDirection::{Up, Down, Left, Right}`, once per directional event |
| `on_pointer` | `PointerEvent { event, x, y }` for native pointer press/click/release events, in display pixels |
| `on_long_press` | Stored message, cloned on LONG_PRESSED; excludes LONG_PRESSED_REPEAT |
| `on_scroll` | `ScrollOffset { x, y }`, actual offset in pixels on continuous SCROLL |
| `on_bubbled_event` | `&InputEvent` copied from the original descendant target |

Callbacks copy input data and enqueue messages. `App::update` still runs only
in `Ui::drain`. Closures must not block, panic, delete native objects or reenter
LVGL; a panic in an `extern "C"` callback aborts. The queue retains its existing
bounded, oldest-message-drop policy. Missing native key/rotary parameters and
pointer events without a pointer device do not fabricate typed payloads.

`event_bubble(bool)` enables LVGL propagation on a child; all intervening
children must propagate for an ancestor's `on_bubbled_event` to observe it.
Direct `on_click`, `on_value_changed` and `on_event` remain local, so a parent's
widget-specific value getter never reads a different child's widget type.
`gesture_bubble(bool)` follows LVGL's separate gesture target selection: when
the native target becomes an ancestor, that ancestor receives its own gesture.

`scroll_dir(ScrollDir)`, `scroll_snap_x/y(ScrollSnap)`,
`scrollbar_mode(ScrollbarMode)`, `scroll_momentum(bool)` and
`scroll_elastic(bool)` override native settings. Unset modifiers leave the
existing setting unchanged; explicit false/None enum variants reset it.
`scroll_to(x, y)` controls the absolute pixel offset on every render after
layout. Values must fit LVGL's `LV_COORD_MIN..=LV_COORD_MAX` coordinate range;
invalid descriptors fail before native mutation. LVGL clips valid offsets to
the actual content bounds. Omitting `scroll_to` releases control. Programmatic
scrolls also emit `on_scroll`, so handlers should use the actual reported offset.

### Optional keypad and focus groups

Enable `input-keypad` on `rs_lvgl_ui` (or on `rs_lvgl` for the driver alone). It adds
Rust allocation support without changing default features or native headers.
`Ui::bind_keypad`, unsafe `Ui::bind_input` for platform keypad/encoder pointers,
or `Ui::enable_focus` activates one group per UI. The global default group is
unchanged. Merely enabling the feature does not focus touch-only screens.

After activation, managed interactive nodes join in view preorder followed by
overlay preorder. LVGL class defaults determine membership, with
`View::focusable(true/false)` overrides. Hidden/disabled nodes and descendants
of hidden/disabled containers are excluded. Opaque custom descendants and
implicit native children are not enrolled, except the managed Pinyin keyboard.
`View::modal(true)` restricts membership to a modal subtree; `Dialog::view` marks
its backdrop automatically. See [grid and modal focus](#grid-navigation-and-modal-focus).

`Ui::focus(Key)` requires a unique non-Auto key across both mounted trees.
Ambiguous or Auto keys return `UiError::InvalidProperty`; missing/ineligible
nodes or an inactive group return `false`. Sibling key identity rules are
unchanged. Reordering preserves a surviving focused object and encoder editing
mode; native reordering may emit focus events. An unchanged update emits none.
`Ui::set_editing(bool)` selects native encoder navigation/edit mode.

The following idle reader can be replaced with a nonblocking board poll.
`display` and the mounted `ui` come from the platform loop above:

```rust,ignore
use rs_lvgl::input_device::{key, Keypad, KeypadInputData};

let keypad = Keypad::register(
    || KeypadInputData::released(key::ENTER),
    &display,
)?;
ui.bind_keypad(&keypad)?;
ui.focus(Key::Named("save"))?;
```

Return `KeypadInputData::pressed(key)` on press and `released(key)` on release,
using the same key for both. `.and_continued()` requests another buffered read.
`key::NEXT/PREV` navigate focus, `key::ENTER` activates, `key::UP/DOWN/LEFT/RIGHT`
operate the focused widget, and `key::ESC/BACKSPACE/DEL/HOME/END` follow native
widget behavior. Other `u32` values pass through, including Unicode code points.
The reader is `'static` and typed separately from the legacy `InputDriver`
trait; the existing `Data` enum and other drivers are unchanged.

Native device deletion releases the reader once and makes `Keypad::raw()`
return `None`. Dropping a live `Keypad` deletes the device. Drop it before LVGL
deinitialization. `Ui::unmount` releases the focus group and detaches devices;
after remount, bind inputs or enable focus again. An unmounted UI can be dropped
after native teardown, preserving the existing lifecycle contract.

The native desktop example supports mouse activation, Tab/Enter navigation,
and scrolling (focus the list with Tab and use arrow keys):

```shell
cargo run -p rs_lvgl_ui --example input --features simulator,input-keypad
```

Set `INPUT_FRAMES` for a bounded run. The sample uses LVGL's native SDL keyboard;
the typed hardware reader is exercised by native integration tests.

## Styles, animation and screens

Additional `Style` fields apply to the same part/state selectors as existing
styles. Omitted fields inherit native/theme values; an explicit zero is retained.
They include `shadow_width/color/opacity/offset_x/offset_y/spread`,
`outline_width/color/opacity/pad`, `bg_grad_color/dir`, `bg_main_stop`,
`bg_grad_stop`, both gradient stop opacities, `text_letter_space`,
`text_line_space`, `text_decoration`, `image_recolor/opacity`,
`transform_scale_x/y`, `transform_rotation`, `transform_pivot_x/y`, and
`blend_mode`. Scale 256 means 100%; rotation uses tenths of a degree.

```rust
use rs_lvgl_ui::prelude::*;
let button: View<()> = button("Focus me")
    .style(
        Style::default()
            .outline_width(0)
            .transition(StyleTransition::new(
                &[TransitionProperty::OutlineWidth],
                150,
            )),
    )
    .state_style(State::FOCUSED, Style::default().outline_width(2));
```

`StyleTransition` retains its native property list and descriptor until detach
or deletion. `Animation::new(property, from, to, duration_ms)` supports `delay`,
`easing`, `repeat` and `reverse`; add it with `View::animate`. Multiple properties
and delays form a timeline. Unchanged descriptors preserve playback; a changed
descriptor restarts that property, and omission cancels it. Changing declarative
styles while animating the same property applies the style immediately; the next
animation tick writes the animated value. Animate floating children when flex/grid
layout would otherwise determine their position. Time values must fit LVGL's
signed animation clock. Native object deletion cancels animations before retained
Rust data is released.

`Screens<M>` manages native screen transitions with one `Ui` per page:

```rust,ignore
let mut screens = Screens::new(&display, Theme::dark())?;
screens.show(&app, ScreenTransition::immediate())?;
screens.show(&next_app, ScreenTransition::new(ScreenEffect::MoveLeft, 220))?;
// Bind inputs to current_mut() after each show.
screens.timer_handler(); // also collects outgoing pages after animation
screens.current_mut().unwrap().drain(&mut app, 64)?;
screens.unmount();
```

The controller retains outgoing pages, handles interrupted transitions and
restores the original screen on unmount. LVGL suspends input during transitions;
outgoing groups are detached and outgoing overlays hidden. Application state
remains outside the controller. A failed new mount preserves the current page.
The runtime rejects a second controller for a claimed display with `InvalidProperty`.
Unmount releases the claim; a later `show` must reacquire it. Do not load/delete its screens through
another controller or raw API. The display and original screen must remain live
until `unmount` or Drop. The capability gallery uses a pending destination to
avoid rerendering the outgoing page before `show`.

## Canvas and snapshots

`widget-canvas` adds `canvas(Canvas::new(width, height))`. Frames support
`background(color, opacity)` and ordered `DrawCommand::Pixel`, `Rect`, `Line`
and `Text`. Drawing clips at the canvas bounds; a single `Pixel` must be in bounds.
Dimensions, aligned native stride and arithmetic are checked before allocation.
The native draw buffer survives every draw task and object reference. Changed
frames redraw the existing object; resize replaces the buffer before releasing
the old one. No platform framebuffer pointer is required.

`Ui::snapshot()` returns an independent `Snapshot { width, height, pixels }`.
Pixels use packed RGBA8888, exactly `width * 4` bytes per row, and remain valid
after UI/native teardown. Capture includes extra draw bounds such as shadows;
top-layer overlays are excluded. Snapshot completion and buffer copying are
synchronous on the UI thread.

## Optional extensions and CJK

These extensions are separate from `widgets-all` and preserve its default native
configuration. Enable only the features needed by the application.

| Feature | Typed API | Native requirement / behavior |
| --- | --- | --- |
| `widget-qrcode` | `qrcode(QrCode::new(bytes, size))` | Canvas/image; length-delimited binary payload, including zero bytes |
| `widget-barcode` | `barcode(Barcode::new(text, height))` | Canvas/image; Raw Code128 preserves spaces; `BarcodeEncoding::Gs1` explicitly selects native GS1 conventions |
| `widget-gif` | `gif(Gif::new(bytes))` | Image + vendored GIF decoder; retained `Rc<[u8]>`, `paused` controls playback |
| `widget-lottie` | `lottie(Lottie::new(json, width, height))` | Canvas/image + vendored ThorVG C++14; checked parse/timing/render results; source or size changes recreate the widget |
| `widget-ime-pinyin` | `pinyin_keyboard(textarea_id, font)` | 26-key keyboard, buttonmatrix, textarea, bundled Pinyin dictionary |
| `font-cjk-14`, `font-cjk-16` | `Font::cjk_14()`, `Font::cjk_16()` | Vendored Source Han Sans SC CJK glyphs |

Lottie enables native vector graphics, matrix support and floating-point precise
coordinates. Its C/C++ compilation and target bindings use the same configuration;
this changes native types for the whole unified feature graph. Prebuilt providers
must match that configuration and are not certified by the host tests. See the
[native build reference](https://github.com/umbrella22/rs_lvgl/blob/main/rs_lvgl_sys/README.md#native-providers-and-extensions).

Native media decoding can reject data after descriptor validation. Such a mount
or update returns `InvalidProperty` and deletes the affected partial tree. Byte
lengths and the native encoded width are checked; identifiers and payloads are
never silently truncated or normalized. GIF data and Canvas/Lottie targets remain
retained until source replacement or native object deletion.

Pinyin binds to a textarea's `keyboard_id`. Removing the target unbinds the native
keyboard before deletion. Use a CJK font for the candidate strip and textarea.
LVGL shares candidate text buffers, so use one active Pinyin keyboard at a time.
FreeType/TinyTTF, ffmpeg/gstreamer and platform texture providers remain separate
integrations.

`components::file_explorer::view(theme, location, parent_message, entries)` composes
file navigation from core nodes. `FileEntry::file/directory(id, name, message)`
keeps display names separate from application identities. The application supplies
entries and performs I/O; this component adds no filesystem dependency.
With `input-keypad` and an explicitly enabled focus group, Tab visits each entry
and Enter emits its activation message. Focus follows entry identity across reorder.

## Grid navigation and modal focus

`input-gridnav` includes `input-keypad` and native GRIDNAV. Add
`View::grid_navigation(GridNavigation { rollover, scroll_first, axis })` to an
Obj, Row, Column or Grid. The direct children form the spatial navigation set;
only the container enters the Tab order. Arrow keys select eligible clickable
children, Enter activates them and Tab moves to the next group member. `Ui::focus`
can select a uniquely keyed direct child. Hidden, disabled and `focusable(false)`
children are excluded. Removing the modifier restores native flags and normal
preorder membership. Empty grids are excluded from the group.

Navigation is limited to direct children. Nested grid scopes, modal scopes inside
a grid, and Pinyin private keyboard targets inside a grid return `InvalidProperty`.
Place independent grids in sibling containers and dialogs in `App::overlay`.

With `input-keypad`, `View::modal(true)` restricts focus to the last visible,
enabled modal subtree in view-then-overlay preorder. Nested scopes restore the
previous scope and focus on close. Restoration uses mounted identity, including
auto-keyed nodes, and restores editing mode only if that identity remains eligible;
otherwise native group order supplies a fallback. Recreating a node with the same
key does not revive its previous identity. The marker traps keyboard/encoder focus;
pointer interception requires a backdrop. `Dialog::view` supplies both.

All focus behavior still requires input binding or `Ui::enable_focus()`.
Enabling features or constructing a modal/grid descriptor alone does not activate
focus on existing touch-only screens.

## Capability gallery

```shell
cargo run -p rs_lvgl_ui --example capabilities --features capability-gallery
```

The SDL window shows styles/motion, Canvas, QR/barcode, GIF/Lottie, CJK/Pinyin,
application-supplied file entries, modal focus and native page transitions.
Use Tab, arrows and Enter, or the mouse. The Snapshot action captures an owned
image in memory and reports its dimensions. Quit releases screens and input devices
before deleting the native display. `UI_FRAMES=900 UI_AUTORUN=1` runs a bounded
walkthrough with the same cleanup path. Closing the SDL title bar follows SDL's
native display-deletion path; the example avoids accessing stale UI handles.

## Lifecycle

Call `Ui::unmount()` before deleting the native display, before `rs_lvgl::deinit()`,
and before dropping the `Display` handle used to construct the `Ui`. Dropping a
`Ui` also deletes its root container, but it must not be the last live LVGL user
during display teardown. A display deletion requires all rendering and pending
flushes to finish first.

A reconcile error after validation (for example an LVGL allocation failure)
deletes the affected mounted tree. The next update recreates it from `App::view`.
Structural and range validation errors precede mutation; native media decoders
can return `InvalidProperty` after creation, with the same partial-tree cleanup.

## Raw escape

`raw` creates an opaque native subtree. The reconciler does not inspect or
reconcile its children, and its callbacks and deletion belong to the raw widget
implementation. The factory is `unsafe`; it must return a live direct child of
the supplied parent that LVGL deletes with the mounted subtree. A raw node has
no update hook and is never visited by `Ui::tick`.

## Custom widget lifecycle

`custom` mounts a widget that owns one native subtree and has a lifecycle:

```rust
pub trait CustomWidget: 'static {
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError>;
    fn update(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError>;
    fn delete(&mut self, object: &mut CustomObject);
}

pub fn custom<M>(widget: impl CustomWidget) -> View<M>;

impl<M: Clone + 'static> Ui<M> {
    pub fn tick(&mut self, elapsed_ms: u32) -> Result<(), UiError>;
}
```

`CustomObject` is an opaque handle to the mounted native object. `obj()` returns
a lifetime-bound `Obj` view, `raw()` exposes the pointer for native calls, and
`invalidate()` marks the subtree dirty and counts the invalidation. A no-op
frame returns from `update` without invalidating; LVGL then keeps the previous
frame.

Lifecycle rules:

- `create` runs once when the node mounts. It must return a handle built with
  the unsafe `CustomObject::new` for a direct child of the supplied parent; the
  reconciler re-checks the parent and fails the mount with `InvalidProperty`
  otherwise. The handle must not be duplicated or retained: the native object
  is deleted right after `delete` returns.
- `Ui::tick` walks the mounted tree once, pre-order, and calls `update` for each
  custom widget, skipping subtrees without lifecycle widgets. It does not call `App::view`, reconcile descriptors or process
  messages. A tick error stops the walk; the tree stays mounted.
- `delete` runs exactly once before the native object is deleted: on key or kind
  change, subtree removal, reconcile rollback and `Ui::unmount`.
- All three hooks run on the UI task inside the calling `Ui` method. They may
  mutate only their own native subtree. Reentrant `tick`, `mount`, `drain`,
  `update` and `post` calls return `UiError::Reentrant`; a nested
  `timer_handler` returns `0`.
- The mounted tree retains the widget instance. A new `custom` value from
  a later `App::view` is used only when the node key or custom variant changes;
  otherwise it is dropped. Keep shared application state in an `Rc`/`RefCell`
  that both the widget and the app can see, or change the key to swap
  implementations.
- Messages from native callbacks enter through the node's explicit handler
  (`on_event`, `on_click`, ...), which the reconciler registers on the custom
  root object. Widgets never reconcile children or mutate siblings.

The trait is not generic over the message type. `WidgetKind` is not generic, so
`CustomWidget<M>` would have forced a generic widget descriptor across the
crate. The explicit node handler provides the same message path without that
refactor.

## Migration notes

### Function constructors

Import `rs_lvgl_ui::prelude::*`, or import the functions and types explicitly
from the crate root. Replace `Node::label(text)` with `label(text)`, and use
`View<Msg>` in view signatures. `View<M>` is an alias of the existing `Node<M>`
descriptor, so modifiers, `.map`, keys and ownership retain their behavior.
Associated constructors on both `Node` and `View` are removed.

Layouts accept their children directly: `row([first, second])`,
`column(items.iter().map(item_view))`, and `grid(columns, rows, children)`.
Use an empty array for an empty row or column. `.child` and `.children` remain
available for incremental assembly. `obj()` remains a plain container, and
`tabview(tabs, active)` / `tileview(tiles, active)` use the existing child
builders for their pages. Optional functions keep their existing feature gates;
`raw(factory)` still requires `unsafe` and its documented lifetime obligations.

### Earlier changes

Changes since the first `rs_lvgl_ui` release that can affect existing code:

- `CustomObject::new` is now `unsafe`. Wrap the call in `unsafe { .. }` and
  uphold its `# Safety` contract (return the handle from `create`; do not keep
  a second handle or the raw pointer past `delete`).
- `ImageSource::Raw` now holds `RawImage` instead of a raw pointer. Build raw
  sources with `ImageSource::from_raw`; read the pointer with
  `RawImage::as_ptr`.
- `Ui::drain` no longer reconciles when no message was processed. Call
  `Ui::mark_dirty()` (or `Ui::update`) after changing state outside messages.
- Combined handlers no longer replace each other. `Handlers` is kept for source
  compatibility but is no longer used by `View`.
- `obj()` with a handler is no longer forced clickable (LVGL objects are
  clickable by default); use `View::clickable(false)` to opt out.
- Plain containers and the root no longer carry the default theme's card style,
  and labels now inherit text color and font. Views that relied on the theme
  card look should style the container explicitly (for example
  `Style::surface(theme)`).
- `bg_opacity` no longer touches the background color, and `bg` sets an opaque
  fill. Write `.bg(color).bg_opacity(opa)` for a translucent color.
- Duplicate `keyboard_id` values anywhere in the tree and invalid part styles
  are now rejected with `UiError::InvalidProperty`.
- `WidgetKind` is now `#[non_exhaustive]`; exhaustive matches need a `_` arm.
- Controlled values follow the native widget: a user change the application
  does not accept is reverted on the next render instead of persisting.
- New `WidgetKind` variants (`Roller`, `Spinbox`, `Spinner`, `Led`, `Line`,
  `ButtonMatrix`, `Table`, `Tileview`, `Msgbox`, `Calendar`, `Spans`,
  `ImageButton`, `AnimImage`, `ArcLabel`) only affect code that matches the
  enum, which already needs a `_` arm.
- `App` gained `overlay`, a provided method returning `None`; existing `App`
  implementations compile unchanged. An implementation that already had an
  inherent method named `overlay` keeps calling it through `self.overlay()`;
  rename it if it should not shadow the trait method in generic code.
- The `components` feature is on by default. Builds with
  `--no-default-features` must add it to keep `rs_lvgl_ui::components`.
- `--no-default-features` now also removes the optional widgets and the
  non-default fonts. Add `widgets-all` and `fonts-vendored` (or the individual
  `widget-<name>`/`font-montserrat-<size>` features) to keep the previous
  set. Without `font-montserrat-28`, `Font::montserrat_28` does not exist and
  the built-in themes use Montserrat 14 for titles.
- `Style::neutral()` does not set `offset` or `floating`. An unset `floating`
  clears `LV_OBJ_FLAG_FLOATING` on mounted nodes; set it through
  `Style::floating` rather than through raw LVGL calls.

## Examples and tests

The [host validation guide](https://github.com/umbrella22/rs_lvgl/blob/main/docs/testing.md) describes the supported matrix,
native example audit and strict Rust/LVGL heap accounting. Run the leak harness
through `python3 scripts/check-ui-leaks.py`; running its opt-in test with the
default CLIB allocator fails because that allocator's monitor is unobservable.
This measurement covers repeated lifecycles after one declared priming cycle,
with explicit cleanup of LVGL's global radius-mask cache. Third-party libc/C++
allocations require separate instrumentation.

```sh
cargo run -p rs_lvgl_ui --example settings
GALLERY_FRAMES=300 cargo run -p rs_lvgl_ui --example gallery --features simulator
DEMO_FRAMES=240 cargo run -p rs_lvgl_ui --example widgets_demo --features simulator
COMPONENTS_FRAMES=240 cargo run -p rs_lvgl_ui --example components --features simulator
cargo test -p rs_lvgl_ui -- --test-threads=1
cargo check -p rs_lvgl_ui --no-default-features --features alloc
cargo check -p rs_lvgl_ui --no-default-features --features alloc,widget-slider
cargo check -p rs_lvgl_ui --target riscv32imafc-unknown-none-elf \
  --no-default-features --features alloc,no-autoinit
```

Cross-compiling needs a RISC-V GCC and its newlib headers. `cc-rs` accepts a
toolchain whose prefix differs from the Rust target name through `CC_<target>`
and `AR_<target>` (with `-` replaced by `_`), and bindgen finds newlib headers
through `LVGL_SYSROOT`:

```sh
export CC_riscv32imafc_unknown_none_elf=riscv64-unknown-elf-gcc
export AR_riscv32imafc_unknown_none_elf=riscv64-unknown-elf-ar
export LVGL_SYSROOT="$(riscv64-unknown-elf-gcc -print-sysroot)"
```

Host tests use a memory display and run serially because LVGL owns global native
state. The gallery uses LVGL's native SDL2 backend, so SDL events are handled in
C and are unaffected by SDL2-compat/SDL3 event-code differences in the Rust
`sdl2` bindings. The crate has no Rust `sdl2` dependency at all. Set
`GALLERY_FRAMES` for a bounded, non-interactive smoke run.

Run the example audit from the repository root (Python 3 and SDL2 required):

```sh
python3 scripts/check-ui-examples.py --artifacts target/ui-audit
# Add --locked --offline when using an existing resolved lock and local cache.
# Headless Linux:
xvfb-run -a python3 scripts/check-ui-examples.py --artifacts target/ui-audit
```

This builds the focused examples and galleries, runs serial regressions against
their actual views, and performs bounded native runs, including every focused
example, each widgets-demo tab and the capability-gallery walkthrough. Tests check rendered focus rings on all four
sides, keyboard/modal focus restoration, pressed/disabled/checked behavior,
native pointer scrolling, keyed list identity, text keyboard binding, theme
changes, and queued updates during native window deletion. `images/` contains
PPM render diagnostics; focused examples also check content bounds and flat
button shadows in both themes.
`runs.json` and logs record each process result. Native
memory-display input tests and bounded SDL runs do not certify OS input delivery
or physical devices. The CI UI job runs these checks under Xvfb, tests combined
optional capabilities, and checks independent reduced feature selections.

The widgets demo updates LVGL's native theme alongside application tokens when
changing appearance or accent. Its retained raw calendar keeps native sizing
and inherits theme colors, so it follows later theme changes. Pointer-only
examples keep explicit keyboard focus disabled even when built with `input-keypad`.

SDL's title-bar close deletes the native display during event dispatch. The
examples detect that before draining queued UI updates and retain their existing
emergency exit without dropping stale Rust tree handles. Use Quit or a frame
limit to exercise normal unmount and resource teardown.

The `widgets_demo` example recreates LVGL's official widgets demo with typed
nodes and a raw calendar. The [component studio](https://github.com/umbrella22/rs_lvgl/blob/main/docs/component-studio.md)
lists widget coverage and interactions.

## Component studio

Run `cargo run -p rs_lvgl_ui --example studio --features showcase` for the complete
component studio. [Coverage and rendering](https://github.com/umbrella22/rs_lvgl/blob/main/docs/component-studio.md) lists its
nine categories and native acceptance checks.
