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
            obj().children(core::iter::once(label("Absolute"))),
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
        typed::bar(0, 100, binding(50, |_: i32| ())).into_view(),
        typed::arc(0, 100, binding(50, |_: i32| ())).into_view(),
        typed::spinbox(0, 100, binding(50, |_: i32| ())).digits(3, 0).step(1).into_view(),
        typed::roller(["a", "b"], 0).visible_rows(3).roller_mode(RollerMode::Normal)
            .on_selection_changed(|_: Option<u32>| ()).into_view(),
        typed::table([["a"]]).column_widths([80]).on_cell_selected(|_: Option<TableCell>| ()).into_view(),
        typed::calendar(2026, 11).today(CalendarDate::new(2026, 11, 5))
            .highlighted([CalendarDate::new(2026, 11, 6)])
            .on_date_selected(|_: Option<CalendarDate>| ()).into_view(),
        typed::msgbox("body").msgbox_title("Title").msgbox_buttons(["OK"])
            .on_selection_changed(|_| ()).into_view(),
        typed::spinner().spinner_anim(1000, 60).into_view(),
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
        .flex_align(FlexAlign::Start, FlexAlign::Center).hidden(false).disabled(false)
        .shadow_width(0).outline_width(0).text_letter_space(0)
        .arc_color(theme.accent).arc_width(2).anim_duration(0)
        .style(style().padding_top(0))
        .state_style(State::PRESSED, style().opacity(200))
}
pub fn from_root() -> rs_lvgl_ui::View<()> {
    rs_lvgl_ui::obj().width(rs_lvgl_ui::px(40)).height(rs_lvgl_ui::content())
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
        check("typed-core-composition", TYPED_CORE)
        check("typed-controls-composition", TYPED_CONTROLS, features=("controls",))
        check("binding-no-std", BINDINGS, features=("controls",))
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
pub fn invalid() { let _ = rs_lvgl_ui::obj::<()>().width(100); }
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
    Ok(image_resource_scaled(ImageResource::rgb565(1, 1, 2, pixels)?, 256))
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
        check("native-dependency-not-typed", '#![no_std]\nuse rs_lvgl_ui::prelude::bar;\n',
              features=("slider",), errors=("E0432",), symbols=("bar",))
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
