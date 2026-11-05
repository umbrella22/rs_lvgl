#!/usr/bin/env python3
"""Check linked layout capabilities in real external wrapper consumers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/layout-config-audit")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--baseline", action="store_true", help="Retain the pre-fix both-disabled oracle")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    fixture = out / "consumer"
    (fixture / "src").mkdir(parents=True, exist_ok=True)
    (fixture / "src/main.rs").write_text("fn main() {}\n")
    manifest = fixture / "Cargo.toml"
    manifest.write_text('[package]\nname="layout-config-consumer"\nversion="0.0.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies]\n'
                        f'rs_lvgl={{path={json.dumps(str(ROOT / "rs_lvgl"))},default-features=false,features=["alloc","unsafe_no_autoinit"]}}\n'
                        f'rs_lvgl_sys={{path={json.dumps(str(ROOT / "rs_lvgl_sys"))}}}\n')
    header = out / "lv_conf.h"
    env = os.environ.copy()
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    env.update(CARGO_TERM_COLOR="never", LVGL_CONFIG_PATH=str(header), CARGO_TARGET_DIR=str(out / "build"))
    flags = ["--offline"] if args.offline else []
    records = []

    def run(name, command, errors=(), symbols=()):
        log = out / (name + ".log")
        start = time.time()
        with log.open("w") as stream:
            proc = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        content = log.read_text(errors="replace")
        passed = (proc.returncode == 0 if not errors else proc.returncode != 0
                  and all(f"error[{error}]" in content for error in errors)
                  and all(symbol in content for symbol in symbols))
        records.append(dict(name=name, command=command, cwd=str(ROOT), start_unix=start,
                            seconds=time.time()-start, exit=proc.returncode, passed=passed,
                            expected_errors=errors, expected_symbols=symbols,
                            log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),
                            header_sha256=hashlib.sha256(header.read_bytes()).hexdigest() if header.exists() else None,
                            lock_sha256=hashlib.sha256((fixture / "Cargo.lock").read_bytes()).hexdigest()))
        (out / "results.json").write_text(json.dumps(records, indent=2) + "\n")
        print(f"{name}: {'PASS' if passed else 'FAIL'}", flush=True)
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{content[-4000:]}")

    subprocess.run(["cargo", "generate-lockfile", "--manifest-path", str(manifest), *flags],
                   cwd=ROOT, env=env, check=True, capture_output=True)
    if args.locked:
        flags.append("--locked")
    base = 'use rs_lvgl::style::{Style, Layout};\nfn main() { let _ = core::mem::size_of::<Layout>(); let mut style = Style::default(); style.set_width(12);'
    flex = 'use rs_lvgl::style::{FlexAlign, FlexFlow}; style.set_layout(Layout::flex()); style.set_flex_flow(FlexFlow::ROW); style.set_flex_grow(1); style.set_flex_main_place(FlexAlign::CENTER);'
    grid = 'use rs_lvgl::style::{GridAlign, CoordDesc}; style.set_layout(Layout::grid()); style.set_grid_column_align(GridAlign::CENTER); let _ = core::mem::size_of::<CoordDesc<2>>();'

    def check(name, source, errors=(), symbols=(), execute=False):
        (fixture / "src/main.rs").write_text(source)
        (out / (name + ".rs")).write_text(source)
        run(name, ["cargo", "run" if execute else "check", "--manifest-path", str(manifest), *flags], errors, symbols)

    for use_flex, use_grid in ((0, 0), (1, 0), (0, 1), (1, 1)):
        name = f"flex-{use_flex}-grid-{use_grid}"
        # These native widgets require Flex independently of this wrapper.
        # Keep the same widget set in all four configurations.
        dependencies = "".join(f"#undef LV_USE_{widget}\n#define LV_USE_{widget} 0\n"
                               for widget in ("LIST", "MENU", "MSGBOX", "TABVIEW", "WIN", "CALENDAR"))
        header.write_text(f'#include {json.dumps(str(ROOT / "rs_lvgl_sys/vendor/include-9.6/lv_conf.h"))}\n'
                          + dependencies +
                          f'#undef LV_USE_FLEX\n#define LV_USE_FLEX ({use_flex} && 1)\n'
                          f'#undef LV_USE_GRID\n#define LV_USE_GRID ({use_grid} + 0)\n')
        if args.baseline:
            check("baseline-no-layout", "fn main() {}", errors=("E0425",), symbols=("lv_style_set_flex", "lv_style_set_grid"))
            return
        source = base + (flex if use_flex else "") + (grid if use_grid else "")
        source += f'assert_eq!(rs_lvgl_sys::LV_USE_FLEX, {use_flex}); assert_eq!(rs_lvgl_sys::LV_USE_GRID, {use_grid}); }}'
        check(name, source, execute=True)
        if not use_flex:
            check(name + "-no-flex-type", 'use rs_lvgl::style::FlexFlow; fn main() {}', ("E0432",), ("FlexFlow",))
            check(name + "-no-flex-layout", 'fn main() { let _ = rs_lvgl::style::Layout::flex(); }', ("E0599",), ("flex",))
            check(name + "-no-flex-style", 'fn main() { rs_lvgl::style::Style::default().set_flex_grow(1); }', ("E0599",), ("set_flex_grow",))
        if not use_grid:
            check(name + "-no-grid-type", 'use rs_lvgl::style::GridAlign; fn main() {}', ("E0432",), ("GridAlign",))
            check(name + "-no-grid-layout", 'fn main() { let _ = rs_lvgl::style::Layout::grid(); }', ("E0599",), ("grid",))
            check(name + "-no-grid-style", 'fn main() { rs_lvgl::style::Style::default().set_grid_cell_column_pos(1); }', ("E0599",), ("set_grid_cell_column_pos",))


if __name__ == "__main__":
    main()
