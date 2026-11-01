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
pub enum Msg { Increment, Child(()) }
pub struct Counter(pub u32);
impl App for Counter {
    type Msg = Msg;
    fn update(&mut self, _: Msg) { self.0 += 1; }
    fn view(&self, theme: &Theme) -> View<Msg> {
        column([
            label(alloc::format!("Count: {}", self.0)),
            row([
                button("Add").style(Style::accent(theme)).on_click(Msg::Increment),
                child().map(Msg::Child),
            ]),
            grid([GridTrack::Fr(1)], [GridTrack::Content], [
                label("Cell").grid_cell(0, 0, 1, 1),
            ]),
            obj().children(core::iter::once(label("Absolute"))),
        ]).style(Style::default().gap(12))
    }
    fn overlay(&self, _: &Theme) -> Option<View<Msg>> { Some(column([])) }
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

SELECTIVE = """#![no_std]
use rs_lvgl_ui::prelude::*;
pub fn from_root() -> View<()> { rs_lvgl_ui::slider(0, 100, 20) }
pub fn from_prelude() -> View<()> { slider(0, 100, 30).on_value_changed(|_| ()) }
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
pub fn policies(ui: &mut Ui<Msg>, font: FontResource) -> View<Msg> {
    let policy = QueuePolicy::new(QueueOverflow::RejectNew).coalesce(|_, _| None);
    let _ = ui.post_with(Msg::Increment, policy);
    ui.set_event_queue_policy(policy);
    ui.set_event_queue_observer(Some(alloc::rc::Rc::new(|_: PostOutcome<Msg>| {})));
    let _: QueueStats = ui.queue_stats();
    label_static("loaded font").font_resource(font)
}
pub struct Quiet;
impl App for Quiet {
    type Msg = ();
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
        check("additions-no-std", ADDITIONS)
        check("strict-no-std", ADDITIONS, features=("strict",))
        check("diagnostics-no-std", ADDITIONS + '''
pub fn sink(ui: &mut Ui<Msg>) {
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
