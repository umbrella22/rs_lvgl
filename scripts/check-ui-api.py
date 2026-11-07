#!/usr/bin/env python3
"""Compile real no_std consumers of the function-based UI API and its boundaries."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


ROOT = Path(__file__).resolve().parents[1]

CORE = """#![no_std]
extern crate alloc;
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
pub enum CounterAction { Increment, Child(()) }
pub struct Counter(pub u32);
impl App for Counter {
    type Action = CounterAction;
    fn update(&mut self, _: CounterAction) { self.0 += 1; }
    fn view(&self, theme: &Theme) -> View<CounterAction> {
        column([
            label(alloc::format!("Count: {}", self.0)).font(theme.title_font).text_color(theme.text),
            row([
                button("Add").style(Style::accent(theme)).on_click(CounterAction::Increment),
                child().map(CounterAction::Child),
            ]),
            grid([GridTrack::Fr(1)], [GridTrack::Content], [
                label("Cell").grid_cell(0, 0, 1, 1),
            ]),
            container().children(core::iter::once(label("Absolute"))),
        ]).gap(12)
    }
    fn overlay(&self, _: &Theme) -> Option<View<CounterAction>> { Some(column([])) }
}
fn child() -> View<()> { button("Child").on_click(()) }
pub fn dynamic(items: &[u64]) -> View<()> {
    column(items.iter().map(|id| label(alloc::format!("{id}")).key(Key::Id(*id))))
}
pub fn root_exports() -> rs_lvgl_ui::View<()> {
    rs_lvgl_ui::column([rs_lvgl_ui::row([]), rs_lvgl_ui::button("Root")])
}
pub fn into_view(node: rs_lvgl_ui::Node<()>) -> View<()> { node }
pub fn into_node(view: View<()>) -> rs_lvgl_ui::Node<()> { view }
"""

APP_ACTIONS = """#![no_std]
use rs_lvgl_ui::prelude::*;
#[derive(Clone)]
pub enum FanAction { SetSpeed(i32) }
pub struct Fan;
impl App for Fan {
    type Action = FanAction;
    fn update(&mut self, _action: Self::Action) {}
    fn view(&self, _: &Theme) -> View<Self::Action> {
        button("Set speed").on_click(FanAction::SetSpeed(40))
    }
    fn overlay(&self, _: &Theme) -> Option<View<Self::Action>> { None }
}
pub fn run<A: App<Action = FanAction>>(ui: &mut Ui<FanAction>, app: &mut A) {
    let _ = ui.mount(app);
    let _ = ui.drain(app, 1);
    let _ = ui.update(app);
}
pub fn action_type(action: <Fan as App>::Action) -> FanAction { action }
"""

SELECTIVE = """#![no_std]
use rs_lvgl_ui::prelude::*;
pub fn from_root() -> View<()> { rs_lvgl_ui::slider(0, 100, 20) }
pub fn from_prelude() -> View<()> { slider(0, 100, 30).on_value_changed(|_| ()) }
"""

TYPED_CORE = """#![no_std]
use rs_lvgl_ui::prelude::*;
pub fn composition(show: bool) -> View<()> {
    typed::column([
        typed::label("title").padding(4).into_view(),
        typed::button("save").on_click(()).into_view(),
    ])
    .child(typed::button("single").on_click(()))
    .child_if(show, || typed::label("conditional"))
    .children((0..3).map(|id| typed::label("item").key(Key::Id(id)).into_view()))
    .into_view()
}
"""

TYPED_CONTROLS = """#![no_std]
use rs_lvgl_ui::prelude::*;
#[derive(Clone)] pub enum FieldAction { Enabled(bool), Speed(i32), Text }
#[derive(Clone)] pub enum PageAction { Field(FieldAction) }
pub fn component(show: bool) -> View<PageAction> {
    let field: rs_lvgl_ui::Node<PageAction, typed::kind::Textarea> =
        typed::textarea::<FieldAction>("name").padding(8)
        .map(PageAction::Field).placeholder("Name").password(true);
    let conditional = if show { typed::label("visible").into_view() }
        else { typed::switch(false).on_toggle(FieldAction::Enabled).into_view() };
    typed::column([
        typed::slider(0, 100, 50).on_value_changed(FieldAction::Speed).into_view(),
        conditional,
    ]).child_if(show, || typed::textarea("hint").placeholder("Hint"))
    .map(PageAction::Field).child(field).into_view()
}
"""

BINDINGS = """#![no_std]
extern crate alloc;
use rs_lvgl_ui::prelude::*;
use alloc::string::String;
#[derive(Clone)] pub enum FormAction { Speed(i32), Enabled(bool), Name(String) }
pub struct Form { pub speed: i32, pub enabled: bool, pub name: String }
impl App for Form {
    type Action = FormAction;
    fn update(&mut self, action: FormAction) {
        match action { FormAction::Speed(v) => self.speed = v,
            FormAction::Enabled(v) => self.enabled = v, FormAction::Name(v) => self.name = v }
    }
    fn view(&self, _: &Theme) -> View<FormAction> {
        typed::column([])
            .child(typed::slider(0, 100, binding(self.speed, FormAction::Speed)))
            .child(typed::switch(binding(self.enabled, FormAction::Enabled)))
            .child(typed::checkbox("Enabled", binding(self.enabled, FormAction::Enabled)))
            .child(typed::textarea(binding(self.name.as_str(), |v: &str| FormAction::Name(v.into())))
                .placeholder("Name"))
            .child(typed::textarea(binding(&self.name, |v: &str| FormAction::Name(v.into()))))
            .child(typed::textarea(binding(self.name.clone(), |v: &str| FormAction::Name(v.into()))))
            .into_view()
    }
}
pub fn borrowed_plain(name: &String) -> View<()> {
    typed::column([]).child(typed::textarea(name)).child(typed::textarea(name.as_str()))
        .child(typed::textarea(name.clone())).into_view()
}
"""

TYPED_OPTIONAL = """#![no_std]
use rs_lvgl_ui::prelude::*;
pub fn controls() -> View<()> {
    typed::column([
        typed::progress_bar(0, 100, binding(50, |_: i32| ())).into_view(),
        typed::arc(0, 100, binding(50, |_: i32| ())).into_view(),
        typed::spinbox(0, 100, binding(50, |_: i32| ())).digits(3, 0).step(1).into_view(),
        typed::roller(["a", "b"], 0).visible_rows(3).roller_mode(RollerMode::Normal)
            .on_selection_changed(|_: Option<u32>| ()).into_view(),
        typed::table([["a"]]).column_widths([80]).on_cell_selected(|_: Option<TableCell>| ()).into_view(),
        typed::calendar(2026, 11).today(CalendarDate::new(2026, 11, 5))
            .highlighted([CalendarDate::new(2026, 11, 6)])
            .on_date_selected(|_: Option<CalendarDate>| ()).into_view(),
        typed::message_box("body").message_box_title("Title").message_box_buttons(["OK"])
            .on_selection_changed(|_| ()).into_view(),
        typed::spinner().spinner_animation(1000, 60).into_view(),
        typed::led(Color::rgb(1, 2, 3)).brightness(100).into_view(),
    ]).into_view()
}
"""

FLAT_STYLES = """#![no_std]
use rs_lvgl_ui::prelude::*;
pub fn from_prelude(theme: &Theme) -> View<()> {
    column([label("Title").font(theme.title_font).text_color(theme.text)])
        .gap(12).padding(16).padding_left(0).padding_right(1)
        .padding_top(2).padding_bottom(3)
        .width(percent(100)).height(content()).grow(1)
        .min_width(px(8)).max_width(px(200)).min_height(px(0)).max_height(px(240))
        .background(theme.surface).background_opacity(128).opacity(240)
        .border(theme.accent, 1).radius(8).align(Align::Center)
        .text_align(TextAlign::Center).offset(-1, 2).floating(false)
        .justify_content(FlexAlign::Start).align_items(ItemAlign::Center).align_content(FlexAlign::Center).hidden(false).disabled(false)
        .shadow_width(0).outline_width(0).letter_spacing(0)
        .arc_color(theme.accent).arc_width(2).animation_duration(0)
        .style(style().padding_top(0))
        .state_style(State::PRESSED, style().opacity(200))
}
pub fn from_root() -> rs_lvgl_ui::View<()> {
    rs_lvgl_ui::container().width(rs_lvgl_ui::px(40)).height(rs_lvgl_ui::content())
        .max_width(rs_lvgl_ui::percent(100)).style(rs_lvgl_ui::style().gap(0))
}
pub fn inspect_patch(patch: Style) {
    let _: Color = patch.background_value();
    let _: u8 = patch.background_opacity_value();
    let _: Color = patch.text_color_value();
    let _: i16 = patch.padding_left_value();
    let _: i16 = patch.padding_right_value();
    let _: i16 = patch.padding_top_value();
    let _: i16 = patch.padding_bottom_value();
    let _: u8 = patch.grow_value();
}
"""

UNTRUSTED_NATIVE_OBJECT = """#![no_std]
struct Invalid;
impl rs_lvgl_ui::NativeObject for Invalid {
    fn raw(&self) -> core::ptr::NonNull<rs_lvgl_sys::lv_obj_t> {
        core::ptr::NonNull::dangling()
    }
}
pub fn invalid_parent() { let _ = rs_lvgl_ui::Obj::create(&mut Invalid); }
"""

BORROWED_NATIVE_OBJECT = """#![no_std]
use rs_lvgl_ui::{NativeObject, Obj};
pub struct Borrowed<'a, 'b>(&'a mut Obj<'b>);
// SAFETY: the private field borrows an existing valid Obj; it adds no deletion
// or invalidation path, and raw delegates to the same UI-thread object.
unsafe impl NativeObject for Borrowed<'_, '_> {
    fn raw(&self) -> core::ptr::NonNull<rs_lvgl_sys::lv_obj_t> { self.0.raw() }
}
pub fn extend(parent: &mut Obj<'_>) {
    let mut borrowed = Borrowed(parent);
    let _ = Obj::create(&mut borrowed);
}
"""

ADDITIONS = CORE + """
pub fn static_and_borrowed_text(short: &str) -> View<()> {
    column([label_static("fixed"), button_static("save"), label(short), button(short)])
}
fn is_copy<T: Copy>() {}
pub fn preserved_copy() {
    is_copy::<Font>(); is_copy::<Style>(); is_copy::<Theme>(); is_copy::<ImageSource>();
}
pub fn policies(ui: &mut Ui<CounterAction>, font: FontResource) -> View<CounterAction> {
    let policy = QueuePolicy::new(QueueOverflow::RejectNew).coalesce(|_, _| None);
    let _ = ui.post_with(CounterAction::Increment, policy);
    ui.set_event_queue_policy(policy);
    ui.set_event_queue_observer(Some(alloc::rc::Rc::new(|_: PostOutcome<CounterAction>| {})));
    let _: QueueStats = ui.queue_stats();
    label_static("loaded font").font_resource(font)
}
pub struct Quiet;
impl App for Quiet {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn update_with_render(&mut self, _: ()) -> RenderDecision { RenderDecision::Skip }
    fn view(&self, _: &Theme) -> View<()> { label_static("quiet") }
}
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/ui-api-audit")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--baseline-native-object", action="store_true",
                        help="Record the compile-only pre-fix unsafe trait oracle")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    records = []
    env = os.environ.copy()
    # Keep the fixture target separate from the application's build outputs.
    env["CARGO_TARGET_DIR"] = str(out / "build")

    def run(name, command, errors=(), symbols=()):
        started = time.time()
        result = subprocess.run(command, cwd=ROOT, env=env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=600)
        log = out / f"{name}.log"
        log.write_text(result.stdout)
        passed = (result.returncode == 0 if not errors else
                  result.returncode != 0
                  and all(f"error[{code}]" in result.stdout for code in errors)
                  and all(symbol in result.stdout for symbol in symbols))
        records.append({"name": name, "command": command, "start_unix": started,
                        "seconds": time.time() - started, "exit": result.returncode,
                        "expected_errors": errors, "expected_symbols": symbols,
                        "passed": passed,
                        "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest()})
        (out / "results.json").write_text(json.dumps(records, indent=2) + "\n")
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{result.stdout[-4000:]}")
        print(f"{name}: PASS", flush=True)

    with tempfile.TemporaryDirectory(prefix="ui-api-consumer-", dir=out) as temporary:
        fixture = Path(temporary)
        (fixture / "src").mkdir()
        (fixture / "src/lib.rs").write_text(CORE)
        manifest = fixture / "Cargo.toml"
        manifest.write_text(
            '[package]\nname="ui-api-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[dependencies]\n'
            f'rs_lvgl_ui={{path={json.dumps(str(ROOT / "rs_lvgl_ui"))},'
            'default-features=false,features=["alloc"]}\n'
            f'rs_lvgl_sys={{path={json.dumps(str(ROOT / "rs_lvgl_sys"))}}}\n'
            '[features]\nslider=["rs_lvgl_ui/widget-slider"]\n'
            'controls=["rs_lvgl_ui/widget-slider","rs_lvgl_ui/widget-switch","rs_lvgl_ui/widget-checkbox","rs_lvgl_ui/widget-textarea"]\n'
            'all-widgets=["rs_lvgl_ui/widgets-all"]\n'
            'charts=["rs_lvgl_ui/widget-chart"]\n'
            'image=["rs_lvgl_ui/widget-image"]\n'
            'diagnostics=["rs_lvgl_ui/diagnostics"]\n'
            'strict=["rs_lvgl_ui/strict-properties"]\n')
        flags = ["--offline"] if args.offline else []
        run("resolve", ["cargo", "generate-lockfile", "--manifest-path", str(manifest), *flags])
        flags += ["--locked"] if args.locked else []

        def check(name, source, features=(), errors=(), symbols=()):
            (fixture / "src/lib.rs").write_text(source)
            # Retain the exact fixture even after the temporary project is removed.
            (out / f"{name}.rs").write_text(source)
            command = ["cargo", "check", "--manifest-path", str(manifest), *flags]
            if features:
                command += ["--features", ",".join(features)]
            run(name, command, errors, symbols)

        if args.baseline_native_object:
            check("unsafe-parent-before", UNTRUSTED_NATIVE_OBJECT)
            return
        check("safe-native-impl-rejected", UNTRUSTED_NATIVE_OBJECT,
              errors=("E0200",), symbols=("NativeObject",))
        check("external-unsafe-impl", BORROWED_NATIVE_OBJECT)
        check("core-no-std", CORE)
        check("custom-props-no-std", (ROOT / "rs_lvgl_ui/tests/api/custom.rs").read_text())
        check("custom-removed-tick-spelling", (ROOT / "rs_lvgl_ui/tests/api/custom.rs").read_text().replace("fn tick(", "fn update("), errors=("E0407",), symbols=("update", "CustomWidget"))
        check("custom-props-required", (ROOT / "rs_lvgl_ui/tests/api/custom.rs").read_text().replace("type Props = GaugeProps;", ""), errors=("E0046",), symbols=("Props",))
        check("custom-removed-one-argument", (ROOT / "rs_lvgl_ui/tests/api/custom.rs").read_text().replace("custom(Gauge, GaugeProps { value: 55 })", "custom(Gauge)"), errors=("E0061",), symbols=("custom",))
        check("custom-wrong-props", (ROOT / "rs_lvgl_ui/tests/api/custom.rs").read_text().replace("custom(Gauge, GaugeProps { value: 55 })", "custom(Gauge, 55u8)"), errors=("E0308",), symbols=("GaugeProps",))
        check("custom-no-parallel-api", '#![no_std]\nuse rs_lvgl_ui::{PropsWidget, custom_with_props};', errors=("E0432",), symbols=("PropsWidget", "custom_with_props"))
        check("typed-core-composition", TYPED_CORE)
        check("chart-no-std", (ROOT / "rs_lvgl_ui/tests/api/chart.rs").read_text(), features=("charts",))
        check("chart-removed-overload", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad(){let _=chart::<()>(ChartType::Line,0,100,[]);}',features=("charts",),errors=("E0061",),symbols=("chart",))
        check("chart-removed-series-values", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad(s:ChartSeries){let _=s.values;}',features=("charts",),errors=("E0609",),symbols=("values",))
        for name in ("chart", "ChartSpec", "ChartSeries", "ChartHistory", "ChartCursor"):
            check("chart-disabled-"+name, '#![no_std]\nuse rs_lvgl_ui::'+name+';',errors=("E0432",),symbols=(name,))
        check("chart-no-parallel-api", '#![no_std]\nuse rs_lvgl_ui::chart_with;',features=("charts",),errors=("E0432",),symbols=("chart_with",))
        check("flex-flow-no-std", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn screen() -> View<()> { typed::row::<()>([]).flex_wrap(true).map(|()| ()).flex_reverse(true).into_view() }\npub fn column_flow() -> View<()> { column::<()>([]).flex_wrap(true).flex_reverse(false) }')
        for name, expression in (("label", 'typed::label::<()>("x").flex_wrap(true)'),
                                 ("grid", 'typed::grid::<()>([], [], []).flex_reverse(true)')):
            check("flex-flow-reject-" + name, '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = ' + expression + '; }',
                  errors=("E0277",), symbols=("Supports", "Flex"))

        check("typed-controls-composition", TYPED_CONTROLS, features=("controls",))
        check("binding-no-std", BINDINGS, features=("controls",))
        check("range-slider-no-std", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn screen() -> View<SliderRange> { typed::range_slider(-100, 100, binding(SliderRange::new(-20, 40), |range| range)).map(|range| range).on_range_changed(|range| range).into_view() }\npub fn root() -> View<()> { rs_lvgl_ui::range_slider(0, 100, rs_lvgl_ui::SliderRange { lower: 0, upper: 100 }) }', features=("controls",))
        check("range-slider-mapper-type", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::range_slider::<()>(0, 100, binding(SliderRange::new(20, 40), |_: i32| ())); }',
              features=("controls",), errors=("E0631",), symbols=("RangeInput",))
        check("range-slider-wrong-widget", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::slider::<()>(0, 100, 20).on_range_changed(|_| ()); }',
              features=("controls",), errors=("E0277",), symbols=("Supports", "Range"))
        check("range-slider-numeric-handler", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::range_slider::<()>(0, 100, SliderRange::new(20, 40)).on_value_changed(|_| ()); }',
              features=("controls",), errors=("E0277",), symbols=("Supports", "Numeric"))
        for name in ("range_slider", "SliderRange"):
            check("range-disabled-" + name, '#![no_std]\nuse rs_lvgl_ui::' + name + ';',
                  errors=("E0432",), symbols=(name,))

        check("text-options-no-std", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn field() -> View<()> { typed::textarea("中é").accepted_chars("中é").input_max_length(core::num::NonZeroU32::new(2)).cursor_position(Some(1)).map(|()| ()).cursor_click_position(false).into_view() }', features=("controls",))
        input_payload = '#![no_std]\nextern crate alloc;\nuse rs_lvgl_ui::{WidgetKind, TextareaInputOptions};\npub fn descriptor() -> WidgetKind { let input: TextareaInputOptions = Default::default(); WidgetKind::Textarea { text: alloc::string::String::new(), placeholder: alloc::string::String::new(), one_line: true, password: false, input: Some(alloc::boxed::Box::new(input)), } }'
        check("textarea-input-payload", input_payload, features=("controls",))
        check("textarea-removed-field-layout", input_payload.replace("input: Some(alloc::boxed::Box::new(input)),", ""),
              features=("controls",), errors=("E0063",), symbols=("input", "Textarea"))
        check("textarea-input-payload-disabled", '#![no_std]\nuse rs_lvgl_ui::TextareaInputOptions;',
              errors=("E0432",), symbols=("TextareaInputOptions",))
        check("text-options-wrong-widget", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::slider::<()>(0, 100, 0).accepted_chars("abc"); }', features=("controls",), errors=("E0277",), symbols=("Supports", "Text"))
        check("text-options-zero-explicit", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::textarea::<()>("").input_max_length(Some(0)); }', features=("controls",), errors=("E0308",), symbols=("NonZero",))
        check("text-options-disabled", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = label::<()>("").accepted_chars("abc"); }', errors=("E0599",), symbols=("accepted_chars",))
        check("typed-optional-properties", TYPED_OPTIONAL, features=("all-widgets",))
        for name, expression, error, symbols in (
            ("numeric-value", 'typed::slider::<()>(0, 100, binding(false, |_: bool| ()))', "E0277", ("NumericInput",)),
            ("toggle-mapper", 'typed::switch::<()>(binding(false, |_: i32| ()))', "E0631", ("ToggleInput", "bool")),
            ("text-mapper", 'typed::textarea::<()>(binding("name", |_: i32| ()))', "E0631", ("TextInput", "str")),
        ):
            check(f"binding-reject-{name}", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = ' + expression + '; }',
                  features=("controls",), errors=(error,), symbols=symbols)
        check("typed-optional-disabled", '#![no_std]\nuse rs_lvgl_ui::typed::slider;\n',
              errors=("E0432",), symbols=("slider",))
        for name, expression, capability in (
            ("switch-textarea-property", 'typed::switch::<()>(false).padding(4).placeholder("wrong")', "Text"),
            ("slider-textarea-property", 'typed::slider::<()>(0, 100, 50).password(true)', "Text"),
            ("textarea-roller-property", 'typed::textarea::<()>("text").visible_rows(3)', "Roller"),
            ("label-children", 'typed::label::<()>("leaf").child(label("child"))', "Children"),
            ("slider-toggle-handler", 'typed::slider::<()>(0, 100, 50).on_toggle(|_| ())', "Toggle"),
            ("switch-numeric-handler", 'typed::switch::<()>(false).on_value_changed(|_| ())', "Numeric"),
        ):
            check(f"typed-reject-{name}", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = ' + expression + '; }',
                  features=("all-widgets",), errors=("E0277",), symbols=("Supports", capability))
        check("typed-mixed-array-needs-conversion", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = column([typed::label::<()>("label"), typed::switch(false)]); }',
              features=("controls",), errors=("E0308",), symbols=("Label", "Switch"))
        check("clarity-names", '#![no_std]\nextern crate alloc;\nuse rs_lvgl_ui::prelude::*;\npub fn screen() -> View<()> {\n    container().scroll_direction(ScrollDirection::HORIZONTAL | ScrollDirection::VERTICAL)\n        .align(Align::TopCenter).child(typed::container())\n        .child(typed::progress_bar(0, 100, 25))\n        .child(typed::message_box("Notice").message_box_title("Title").message_box_buttons(["OK"]))\n        .child(typed::label("Long text").text_overflow(TextOverflow::Ellipsis))\n        .child(typed::animated_image([], 1000).repeat(ANIMATION_REPEAT_INFINITE))\n        .child(typed::spinner().spinner_animation(1000, 90))\n        .native_part_style(NativePart::Main, Style::plain().animation_duration(100))\n}\npub fn markers(_: rs_lvgl_ui::Node<(), typed::kind::Container>,\n    _: rs_lvgl_ui::Node<(), typed::kind::ProgressBar>,\n    _: rs_lvgl_ui::Node<(), typed::kind::MessageBox>,\n    _: rs_lvgl_ui::Node<(), typed::kind::AnimatedImage>) {}\n', features=("all-widgets",))
        for old in ("obj", "bar", "msgbox", "anim_image", "ScrollDir", "LabelLongMode", "Part", "ANIM_IMAGE_MAX_FRAMES", "ANIM_REPEAT_INFINITE"):
            for namespace in ("", "prelude::"):
                check(f"clarity-removed-{namespace.replace('::', '') or 'root'}-{old}",
                      f'#![no_std]\nuse rs_lvgl_ui::{namespace}{old};\n',
                      features=("all-widgets",), errors=("E0432",), symbols=(old,))
        for old in ("obj", "bar", "msgbox", "anim_image"):
            check(f"clarity-removed-typed-{old}", f'#![no_std]\nuse rs_lvgl_ui::typed::{old};\n',
                  features=("all-widgets",), errors=("E0432",), symbols=(old,))
        for old, arg in (("neutral", ""),):
            check(f"clarity-removed-style-{old}", f'#![no_std]\npub fn old() {{ let _ = rs_lvgl_ui::Style::{old}({arg}); }}\n',
                  errors=("E0599",), symbols=(old,))
        check("typed-label-overflow-only", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = typed::button::<()>("Save").text_overflow(TextOverflow::Clip); }',
              errors=("E0277",), symbols=("Supports", "Label"))
        check("semantic-units-no-std", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn view() -> View<()> {\n    container().transform_scale_x(Scale::percent(150))\n        .transform_rotation(Angle::degrees(-45))\n        .animate(Animation::rotation(Angle::degrees(0), Angle::tenths(455), 200))\n        .animate(Animation::scale_y(Scale::percent(100), Scale::ratio(3, 2).unwrap(), 200))\n        .child(typed::image_scaled(ImageSource::rgb565(1, 1, &[0, 0]), Scale::percent(50)))\n}\n', features=("image",))
        for name, expression, ty in (
            ("scale", 'container::<()>().transform_scale_x(256)', "Scale"),
            ("angle", 'style().transform_rotation(900)', "Angle"),
            ("image", 'image_scaled::<()>(ImageSource::rgb565(1, 1, &[0, 0]), 256)', "Scale"),
            ("typed-image", 'typed::image_scaled::<()>(ImageSource::rgb565(1, 1, &[0, 0]), 256)', "Scale"),
            ("animation-angle", 'Animation::rotation(0, 900, 200)', "Angle"),
            ("animation-scale", 'Animation::scale_x(256, 512, 200)', "Scale"),
        ):
            check(f"reject-raw-unit-{name}", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = ' + expression + '; }',
                  features=("image",), errors=("E0308",), symbols=(ty,))
        for variant in ("Rotation", "ScaleX", "ScaleY"):
            check(f"removed-raw-animation-{variant}", f'#![no_std]\npub fn bad() {{ let _ = rs_lvgl_ui::AnimationProperty::{variant}; }}',
                  errors=("E0599",), symbols=(variant,))
        check("independent-layout-no-std", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn good() -> View<()> { typed::row([]).justify_content(FlexAlign::SpaceBetween).align_items(ItemAlign::Center).style(style().align_content(FlexAlign::End)).into_view() }')
        check("item-distribution-rejected", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = row::<()>([]).align_items(FlexAlign::SpaceBetween); }',
              errors=("E0308",), symbols=("ItemAlign", "FlexAlign"))
        check("removed-coupled-flex-align", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = row::<()>([]).flex_align(FlexAlign::Start, FlexAlign::Center); }',
              errors=("E0599",), symbols=("flex_align",))
        check("semantic-parts", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn view() -> View<()> { container().child(typed::slider(0, 100, 50).thumb_style(style().radius(8)).progress_style(style().opacity(160))).child(typed::switch(false).thumb_state_style(State::CHECKED, style().radius(4))).child(typed::progress_bar(0, 100, 50).progress_state_style(State::PRESSED, style().opacity(80))) }', features=("all-widgets",))
        for name, expression, capability in (
            ("bar-thumb", 'typed::progress_bar::<()>(0, 100, 50).thumb_style(style())', "Thumb"),
            ("switch-progress", 'typed::switch::<()>(false).progress_style(style())', "Progress"),
            ("arc-progress", 'typed::arc::<()>(0, 100, 50).progress_state_style(State::PRESSED, style())', "Progress"),
            ("label-thumb", 'typed::label::<()>("text").thumb_state_style(State::PRESSED, style())', "Thumb"),
        ):
            check(f"typed-reject-semantic-{name}", '#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn bad() { let _ = ' + expression + '; }',
                  features=("all-widgets",), errors=("E0277",), symbols=("Supports", capability))
        for old, argument in (
            ("bg_grad_color", "Color::rgb(1, 2, 3)"), ("bg_grad_dir", "GradientDirection::Vertical"),
            ("bg_main_stop", "0"), ("bg_grad_stop", "255"), ("bg_main_opacity", "255"), ("bg_grad_opacity", "255"),
            ("text_letter_space", "2"), ("text_line_space", "3"), ("outline_pad", "1"),
            ("image_recolor", "Color::rgb(1, 2, 3)"), ("image_recolor_opacity", "255"), ("anim_duration", "200"),
        ):
            for target in ("style()", "container::<()>()"):
                check(f"removed-clarity-method-{old}-{'style' if target == 'style()' else 'node'}",
                      f'#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn old() {{ let _ = {target}.{old}({argument}); }}',
                      errors=("E0599",), symbols=(old,))
        for old, argument in (("scroll_dir", "ScrollDirection::VERTICAL"), ("part_style", "NativePart::Main, style()"),
                              ("part_state_style", "NativePart::Main, State::PRESSED, style()"),
                              ("spinner_anim", "1000, 90"), ("msgbox_title", '"title"'), ("msgbox_buttons", '["ok"]')):
            check(f"removed-clarity-node-{old}", f'#![no_std]\nuse rs_lvgl_ui::prelude::*;\npub fn old() {{ let _ = container::<()>().{old}({argument}); }}',
                  features=("all-widgets",), errors=("E0599",), symbols=(old,))
        for old in ("Obj", "Bar", "Msgbox", "AnimImage"):
            check(f"removed-clarity-marker-{old}", f'#![no_std]\nuse rs_lvgl_ui::typed::kind::{old};',
                  features=("all-widgets",), errors=("E0432",), symbols=(old,))
        for ty, variants in (("Align", ("TopMid", "BottomMid", "LeftMid", "RightMid")),
                             ("TextOverflow", ("Dots", "ScrollCircular")),
                             ("ScrollDirection", ("HOR", "VER")),
                             ("TransitionProperty", ("ImageRecolor", "ImageRecolorOpacity", "OutlinePad", "LetterSpace", "LineSpace", "GradientColor"))):
            for old in variants:
                check(f"removed-clarity-{ty}-{old}", f'#![no_std]\npub fn old() {{ let _ = rs_lvgl_ui::{ty}::{old}; }}',
                      errors=("E0599",), symbols=(old,))
        check("app-action-associated-type", APP_ACTIONS)
        check("app-msg-associated-type-removed",
              APP_ACTIONS.replace("type Action = FanAction;", "type Msg = FanAction;"),
              errors=("E0437", "E0046"), symbols=("Msg", "Action"))
        check("app-msg-projection-removed", APP_ACTIONS + '''
pub fn old(_: <Fan as App>::Msg) {}
''', errors=("E0576",), symbols=("Msg", "App"))
        check("app-action-type-mismatch", APP_ACTIONS + '''
pub fn wrong(app: &mut Fan) { app.update(()); }
''', errors=("E0308",), symbols=("FanAction",))
        check("flat-styles-no-std", FLAT_STYLES)
        check("flat-styles-strict-no-std", FLAT_STYLES, features=("strict",))
        check("explicit-size-unit-required", '''#![no_std]
pub fn invalid() { let _ = rs_lvgl_ui::container::<()>().width(100); }
''', errors=("E0308",), symbols=("Size",))
        for old, argument in (
            ("bg", "rs_lvgl_ui::Color::rgb(1, 2, 3)"),
            ("bg_opacity", "128"),
            ("fg", "rs_lvgl_ui::Color::rgb(1, 2, 3)"),
            ("pad", "8"), ("pad_left", "8"), ("pad_right", "8"),
            ("pad_top", "8"), ("pad_bottom", "8"), ("flex_grow", "1"),
        ):
            check(f"removed-style-{old}",
                  f'#![no_std]\npub fn old() {{ let _ = rs_lvgl_ui::style().{old}({argument}); }}\n',
                  errors=("E0599",), symbols=(old,))
        check("additions-no-std", ADDITIONS)
        check("strict-no-std", ADDITIONS, features=("strict",))
        check("diagnostics-no-std", ADDITIONS + '''
pub fn sink(ui: &mut Ui<CounterAction>) {
    ui.set_diagnostic_sink(Some(alloc::rc::Rc::new(|_: rs_lvgl_ui::DiagnosticEvent| {})));
}
''', features=("diagnostics",))
        check("loaded-image-no-std", ADDITIONS + '''
pub fn loaded(pixels: alloc::rc::Rc<[u8]>) -> Result<View<()>, ResourceError> {
    Ok(image_resource_scaled(ImageResource::rgb565(1, 1, 2, pixels)?, Scale::percent(100)))
}
''', features=("image",))
        check("loaded-font-requires-unsafe", '''#![no_std]
pub fn transfer(raw: core::ptr::NonNull<rs_lvgl_sys::lv_font_t>,
    destroy: unsafe extern "C" fn(*mut rs_lvgl_sys::lv_font_t)) {
    let _ = rs_lvgl_ui::FontResource::from_raw_owned(raw, (), destroy);
}
''', errors=("E0133",), symbols=("from_raw_owned",))
        for symbol in ("ImageResource", "image_resource", "image_resource_scaled", "DiagnosticSink", "DiagnosticEvent", "WidgetType"):
            check(f"addition-disabled-{symbol}", f'#![no_std]\nuse rs_lvgl_ui::{symbol};\n',
                  errors=("E0432",), symbols=(symbol,))
        check("selective-slider", SELECTIVE, features=("slider",))
        # Recheck the same no_std source after removing the optional capability.
        check("core-after-selective", CORE)
        check("optional-root-disabled", '#![no_std]\nuse rs_lvgl_ui::slider;\n',
              errors=("E0432",), symbols=("slider",))
        check("optional-prelude-disabled", '#![no_std]\nuse rs_lvgl_ui::prelude::slider;\n',
              errors=("E0432",), symbols=("slider",))
        check("native-dependency-not-typed", '#![no_std]\nuse rs_lvgl_ui::prelude::progress_bar;\n',
              features=("slider",), errors=("E0432",), symbols=("progress_bar",))
        for name in ("canvas", "qrcode", "barcode", "gif", "lottie", "pinyin_keyboard"):
            for namespace in ("", "prelude::"):
                check(f"disabled-{namespace.replace('::', '') or 'root'}-{name}",
                      f'#![no_std]\nuse rs_lvgl_ui::{namespace}{name};\n',
                      errors=("E0432",), symbols=(name,))
        check("legacy-constructor-removed",
              '#![no_std]\npub fn old() { let _ = rs_lvgl_ui::Node::<()>::column(); }\n',
              errors=("E0599",), symbols=("column",))
        check("raw-requires-unsafe",
              '#![no_std]\npub fn invalid() { let _: rs_lvgl_ui::View<()> = '
              'rs_lvgl_ui::raw(|_| None); }\n', errors=("E0133",), symbols=("raw",))
    print(f"Passed {len(records) - 1} public UI API contracts.")


if __name__ == "__main__":
    main()
