#!/usr/bin/env python3
"""Verify native display layouts, retained resources and no_std API boundaries."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


ROOT = Path(__file__).resolve().parents[1]

CORE = """#![no_std]
use core::time::Duration;
use rs_lvgl::{Display, DisplayBuffers, DisplayBuilder, PixelBuffer, RenderMode};
pub fn register(bytes: &'static mut [u8]) {
    let _ = DisplayBuilder::new(7, 5).render_mode(RenderMode::Direct).stride(32)
        .invalidation_alignment(4, 1)
        .buffers(DisplayBuffers::Single(PixelBuffer::Static(bytes)))
        .build(|info| { let _ = (info.area, info.stride(), info.origin(), info.is_last()); });
}
pub fn period(display: &mut Display) {
    let _ = display.set_refresh_period(Duration::from_millis(33));
    let _ = display.refresh_period();
    let _ = display.active_buffer();
}
"""

OWNED = """#![no_std]
extern crate alloc;
use rs_lvgl::{CanvasBuffer, ColorFormat, DisplayBuilder, DisplayBuffers, PixelBuffer};
pub fn owned(bytes: alloc::boxed::Box<[u8]>) {
    let _ = DisplayBuilder::new(7, 5)
        .buffers(DisplayBuffers::Single(PixelBuffer::Owned(bytes))).build(|_| {});
}
pub fn canvas(bytes: alloc::boxed::Box<[u8]>) {
    let _ = CanvasBuffer::new(7, 5, ColorFormat::Rgb888, Some(32), PixelBuffer::Owned(bytes));
}
pub fn images(bytes: alloc::rc::Rc<[u8]>) {
    let _ = rs_lvgl_ui::ImageSource::rgb888(1, 1, &[0, 0, 255]);
    let _ = rs_lvgl_ui::ImageSource::rgb888_with_stride(1, 1, 4, &[0, 0, 255, 0]);
    let _ = rs_lvgl_ui::ImageResource::rgb888(1, 1, 4, bytes);
}
"""


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/display-contracts")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--repeat-lifecycle", type=int, choices=(1, 3), default=1)
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    env["CARGO_TERM_COLOR"] = "never"
    flags = (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    records = []
    source_files = [*sorted((ROOT / "rs_lvgl/src/display").glob("*.rs")),
                    ROOT / "rs_lvgl/src/display.rs", ROOT / "rs_lvgl/src/widgets/canvas_buffer.rs",
                    ROOT / "rs_lvgl/tests/display_api.rs", ROOT / "rs_lvgl_ui/tests/display_images.rs",
                    ROOT / "rs_lvgl_ui/src/node/types/image.rs", ROOT / "rs_lvgl_ui/src/resources.rs",
                    ROOT / "rs_lvgl_sys/shims/rs_lvgl_sys.c", ROOT / "rs_lvgl_sys/shims/rs_lvgl_sys.h",
                    Path(__file__).resolve()]
    identities = {
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "toolchain": subprocess.check_output(["rustc", "-vV"], text=True).strip(),
        "provider": "vendored LVGL 9.6.0",
        "lock_sha256": digest(ROOT / "Cargo.lock"),
        "sources": {str(path.relative_to(ROOT)): digest(path) for path in source_files},
    }

    def run(name, command, environment, errors=(), symbols=(), config=None, lock=None, source=None):
        log = out / f"{name}.log"
        start = time.time()
        with log.open("w") as stream:
            proc = subprocess.run(command, cwd=ROOT, env=environment, stdout=stream,
                                  stderr=subprocess.STDOUT, timeout=600)
        content = log.read_text(errors="replace")
        passed = (proc.returncode == 0 if not errors else proc.returncode != 0
                  and all(error in content for error in errors)
                  and all(symbol in content for symbol in symbols))
        records.append(dict(name=name, command=command, cwd="repository root", start_unix=start,
                            seconds=time.time() - start, exit=proc.returncode, passed=passed,
                            expected_errors=errors, expected_symbols=symbols,
                            log_sha256=digest(log), config_sha256=digest(config) if config else None,
                            fixture_lock_sha256=digest(lock) if lock else None,
                            fixture_source_sha256=digest(source) if source else None))
        (out / "results.json").write_text(json.dumps(dict(identities=identities, gates=records), indent=2) + "\n")
        print(f"{name}: {'PASS' if passed else 'FAIL'}", flush=True)
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{content[-5000:]}")

    native = ["cargo", "test", "-p", "rs_lvgl", *flags, "--features", "alloc,input-keypad,rust_timer",
              "--test", "display_api", "--", "--test-threads=1"]
    images = ["cargo", "test", "-p", "rs_lvgl_ui", *flags, "--test", "canvas",
              "--test", "display_images", "--", "--test-threads=1"]
    for iteration in range(1, args.repeat_lifecycle + 1):
        run(f"native-lifecycle-{iteration}", native, env)
        run(f"image-canvas-lifecycle-{iteration}", images, env)

    header = out / "lv_conf.h"
    header.write_text(f'#include {json.dumps(str(ROOT / "rs_lvgl_sys/vendor/include-9.6/lv_conf.h"))}\n'
                      '#undef LV_DRAW_BUF_STRIDE_ALIGN\n#define LV_DRAW_BUF_STRIDE_ALIGN 8\n'
                      '#undef LV_DRAW_BUF_ALIGN\n#define LV_DRAW_BUF_ALIGN 8\n')
    run("aligned-native-layout", native, env | {
        "LVGL_CONFIG_PATH": str(header), "CARGO_TARGET_DIR": str(out / "aligned-build")}, config=header)

    fixture = out / "consumer"
    (fixture / "src").mkdir(parents=True, exist_ok=True)
    manifest = fixture / "Cargo.toml"
    manifest.write_text('[package]\nname="display-contract-consumer"\nversion="0.0.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies]\n'
                        f'rs_lvgl={{path={json.dumps(str(ROOT / "rs_lvgl"))},default-features=false}}\n'
                        f'rs_lvgl_ui={{path={json.dumps(str(ROOT / "rs_lvgl_ui"))},default-features=false,optional=true}}\n'
                        '[features]\nowned=["rs_lvgl/alloc"]\n'
                        'images=["owned","dep:rs_lvgl_ui","rs_lvgl_ui/alloc",'
                        '"rs_lvgl_ui/widget-image","rs_lvgl_ui/widget-canvas"]\n')
    source = fixture / "src/lib.rs"
    source.write_text(CORE)
    external = env | {"CARGO_TARGET_DIR": str(out / "consumer-build")}
    run("consumer-lock", ["cargo", "generate-lockfile", "--manifest-path", str(manifest),
                          *(["--offline"] if args.offline else [])], external, lock=fixture / "Cargo.lock")

    def check(name, code, features=(), errors=(), symbols=()):
        source.write_text(code)
        (out / f"{name}.rs").write_text(code)
        command = ["cargo", "check", "--manifest-path", str(manifest), *flags]
        if features:
            command += ["--features", ",".join(features)]
        run(name, command, external, errors, symbols, lock=fixture / "Cargo.lock", source=source)

    check("static-no-std", CORE)
    check("owned-images-no-std", OWNED, ("images",))
    check("owned-disabled", '#![no_std]\npub fn bad() { let _ = rs_lvgl::PixelBuffer::Owned; }\n',
          errors=("error[E0599]",), symbols=("Owned",))
    check("static-lifetime", '#![no_std]\npub fn bad(bytes: &mut [u8]) -> rs_lvgl::PixelBuffer { '
          'rs_lvgl::PixelBuffer::Static(bytes) }\n',
          errors=("lifetime may not live long enough",), symbols=("static",))
    for name, code, symbol in (
        ("buffer-borrow", 'pub fn bad(buffer: &rs_lvgl::BufferInfo) { let _ = buffer.as_bytes(); }', "as_bytes"),
        ("flush-rows", 'pub fn bad(info: &rs_lvgl::FlushInfo) { let _ = info.rows(); }', "rows"),
        ("async-registration", 'pub fn bad(builder: rs_lvgl::DisplayBuilder) { let _ = builder.build_async(|_| {}, || {}); }', "build_async"),
        ("flush-completion", 'pub fn bad(display: &mut rs_lvgl::Display) { display.flush_ready(); }', "flush_ready"),
    ):
        check(f"reject-safe-{name}", "#![no_std]\n" + code + "\n", errors=("error[E0133]",), symbols=(symbol,))
    check("unsafe-backend-control", CORE + '''
pub unsafe fn backend(builder: DisplayBuilder, info: &rs_lvgl::FlushInfo, display: &mut Display) {
    let _ = builder.build_async(|_| {}, || {});
    let _ = info.rows();
    let _ = info.buffer().as_bytes();
    display.flush_ready();
}
''')
    assert all(digest(ROOT / path) == value for path, value in identities["sources"].items()), "inputs changed during validation"
    assert digest(ROOT / "Cargo.lock") == identities["lock_sha256"], "workspace lock changed"
    print(f"Passed {len(records)} display gates; physical display/DMA acceptance is separate.")


if __name__ == "__main__":
    main()
