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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/ui-api-audit")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
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
            '[features]\nslider=["rs_lvgl_ui/widget-slider"]\n')
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

        check("core-no-std", CORE)
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
