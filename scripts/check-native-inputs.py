#!/usr/bin/env python3
"""Execute raw configuration, custom C/font, rebuild and package consumers."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/native-input-audit")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--locked", action="store_true")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    results = []
    env = os.environ.copy()
    env["CARGO_TERM_COLOR"] = "never"
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    target = out / "build-a"
    env["CARGO_TARGET_DIR"] = str(target)

    def run(name, command, expected=None, overrides=None, cwd=ROOT):
        child_env = env | (overrides or {})
        log = out / (name + ".log")
        start = time.time()
        with log.open("w") as stream:
            completed = subprocess.run(command, cwd=cwd, env=child_env, stdout=stream,
                                       stderr=subprocess.STDOUT, timeout=300)
        content = log.read_text(errors="replace")
        passed = (completed.returncode == 0 if expected is None else
                  completed.returncode != 0 and expected in content)
        results.append(dict(name=name, command=command, exit=completed.returncode,
                            expected_failure=expected, passed=passed, seconds=time.time()-start,
                            start_unix=start, cwd=str(cwd),
                            log_sha256=digest(log)))
        (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print(f"{name}: {'PASS' if passed else 'FAIL'}", flush=True)
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{content[-2500:]}")
        return content

    flags = (["--offline"] if args.offline else [])
    fixture = out / "raw-consumer"
    (fixture / "src").mkdir(parents=True, exist_ok=True)
    config = out / "application config"
    config.mkdir(exist_ok=True)
    extras = out / "custom sources"
    (extras / "private").mkdir(parents=True, exist_ok=True)
    header = extras / "application.h"
    header.write_text('#include "lvgl.h"\nint application_value(void);\n'
                      'int application_dpi(void);\nextern const lv_font_t application_font;\n')
    (config / "base.h").write_bytes((ROOT / "rs_lvgl_sys/vendor/include-9.6/lv_conf.h").read_bytes())
    (config / "lv_conf.h").write_text('#include "base.h"\n#include "settings.h"\n'
                                    '#undef LV_DPI_DEF\n#define LV_DPI_DEF (APP_DPI + 0)\n'
                                    '#undef LV_USE_SLIDER\n#define LV_USE_SLIDER (APP_SLIDER && 1)\n')

    def settings(dpi, slider=0):
        (config / "settings.h").write_text(f"#define APP_DPI {dpi}\n#define APP_SLIDER {slider}\n")

    settings(173)
    (extras / "private/value.h").write_text("#define PRIVATE_VALUE 7\n")
    source = extras / "application.c"
    source.write_text('#include "application.h"\n#include "private/value.h"\n'
                      'int application_value(void) { return PRIVATE_VALUE; }\n'
                      'int application_dpi(void) { return LV_DPI_DEF; }\n'
                      'const lv_font_t application_font = { .line_height = 19 };\n')
    env.update(LVGL_CONFIG_PATH=str(config / "lv_conf.h"), LVGL_EXTRA_C_DIR=str(extras),
               LVGL_EXTRA_HEADERS=str(header))

    def manifest(sys_path):
        (fixture / "Cargo.toml").write_text('[package]\nname="native-input-consumer"\n'
            'version="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\n'
            f'rs_lvgl_sys={{path={json.dumps(str(sys_path))}}}\n')

    manifest(ROOT / "rs_lvgl_sys")
    (fixture / "src/main.rs").write_text('''use rs_lvgl_sys::*;
fn main() {
    let expected: Vec<i32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    assert_eq!(LV_DPI_DEF as i32, expected[0]);
    unsafe {
        assert_eq!(application_dpi(), expected[0]);
        assert_eq!(application_value(), expected[1]);
        assert_eq!(application_font.line_height as i32, expected[2]);
        lv_init();
        let display = lv_display_create(32, 32);
        assert!(!display.is_null());
        lv_display_delete(display);
        lv_deinit();
    }
    println!("RAW_CONFIG_C_FONT_OK");
}
''')
    cargo = ["cargo", "run", "--manifest-path", str(fixture / "Cargo.toml"), *flags]
    run("resolve-raw-lock", ["cargo", "generate-lockfile", "--manifest-path", str(fixture / "Cargo.toml"), *flags])
    if args.locked:
        cargo.append("--locked")
    run("raw-custom", cargo + ["--", "173", "7", "19"])
    cached = run("raw-cached", cargo + ["--", "173", "7", "19"])
    if "Compiling rs_lvgl_sys" in cached:
        results[-1]["passed"] = False
        (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        raise RuntimeError("unchanged input rebuilt rs_lvgl_sys; inspect raw-cached.log")
    settings(181)
    run("transitive-config", cargo + ["--", "181", "7", "19"])
    (extras / "private/value.h").write_text("#define PRIVATE_VALUE 9\n")
    run("private-c-header", cargo + ["--", "181", "9", "19"])
    source.write_text(source.read_text().replace("line_height = 19", "line_height = 21"))
    run("custom-font-source", cargo + ["--", "181", "9", "21"])
    run("feature-conflict", cargo + ["--features", "rs_lvgl_sys/widget-slider", "--", "181", "9", "21"],
        expected="LVGL_CONFIG_PATH conflicts with a requested Cargo capability: LV_USE_SLIDER")
    run("invalid-config-path", cargo, expected="LVGL_CONFIG_PATH must name an existing file",
        overrides={"LVGL_CONFIG_PATH": str(out / "absent.h")})
    (config / "settings.h").write_text("#error INPUT_CONFIG_FAILURE\n")
    run("invalid-header", cargo, expected="INPUT_CONFIG_FAILURE")
    settings(181)
    run("failed-build-recovery", cargo + ["--", "181", "9", "21"])

    config_b = out / "configuration-b.h"
    config_b.write_text(f'#include {json.dumps(str(config / "lv_conf.h"))}\n'
                        '#undef LV_DPI_DEF\n#define LV_DPI_DEF 197\n')
    run("config-path-switch", cargo + ["--", "197", "9", "21"],
        overrides={"LVGL_CONFIG_PATH": str(config_b)})
    run("isolated-output", cargo + ["--", "197", "9", "21"],
        overrides={"LVGL_CONFIG_PATH": str(config_b), "CARGO_TARGET_DIR": str(out / "build-b")})
    run("restore-output-a", cargo + ["--", "181", "9", "21"])

    # A generated compiler wrapper pauses only actual C compilation. Killing
    # its isolated process group leaves the repository and other builds alone.
    if os.name != "posix":
        raise RuntimeError("interruption gate requires POSIX process groups; not-run")
    compiler = shutil.which("cc")
    if not compiler:
        raise RuntimeError("C compiler unavailable")
    marker = out / "compiler-paused"
    marker.unlink(missing_ok=True)
    wrapper = out / "pause-cc"
    wrapper.write_text(f'''#!{sys.executable}
import os, signal, sys
from pathlib import Path
if "-c" in sys.argv and any("lvgl-9.6" in arg for arg in sys.argv):
    Path({str(marker)!r}).write_text("native compilation reached")
    signal.pause()
os.execv({compiler!r}, [{compiler!r}, *sys.argv[1:]])
''')
    wrapper.chmod(0o755)
    settings(183)
    interrupt_log = out / "interrupted.log"
    with interrupt_log.open("w") as stream:
        interrupted_start = time.time()
        child = subprocess.Popen(cargo + ["--", "183", "9", "21"], cwd=ROOT,
            env=env | {"CC": str(wrapper)}, stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + 45
        try:
            while not marker.exists() and child.poll() is None and time.monotonic() < deadline:
                time.sleep(0.1)
            if not marker.exists():
                raise RuntimeError("interruption fixture did not reach native compilation")
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
            child.wait(timeout=10)
    results.append(dict(name="interrupted-compilation", passed=True, exit=child.returncode,
                        command=cargo + ["--", "183", "9", "21"], cwd=str(ROOT),
                        start_unix=interrupted_start, seconds=time.time()-interrupted_start,
                        log_sha256=digest(interrupt_log)))
    run("interrupted-build-recovery", cargo + ["--", "183", "9", "21"])

    # The wrapper build dependency and normal dependency both receive the
    # custom header; generated wrappers must use the linked dependency metadata.
    typed = out / "typed-consumer"
    (typed / "src").mkdir(parents=True, exist_ok=True)
    (typed / "Cargo.toml").write_text('[package]\nname="typed-config-consumer"\nversion="0.0.0"\n'
        'edition="2021"\n[workspace]\n[dependencies]\n'
        f'rs_lvgl={{path={json.dumps(str(ROOT / "rs_lvgl"))},default-features=false,features=["alloc"]}}\n')
    (typed / "src/main.rs").write_text('use rs_lvgl::widgets::Slider;\nfn main(){let _ = core::mem::size_of::<Slider>();}\n')
    typed_command = ["cargo", "check", "--manifest-path", str(typed / "Cargo.toml"), *flags]
    run("disabled-wrapper", typed_command, expected="no `Slider` in `widgets`")
    settings(183, 1)
    run("enabled-wrapper", typed_command + (["--locked"] if args.locked else []))

    # Package from this checkout, then consume only its unpacked source path.
    saved = {key: env.pop(key) for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS")}
    try:
        run("package", ["cargo", "package", "-p", "rs_lvgl_sys", "--allow-dirty", "--no-verify",
                        *flags, *(["--locked"] if args.locked else [])])
    finally:
        env.update(saved)
    archives = list((target / "package").glob("rs_lvgl_sys-*.crate"))
    if len(archives) != 1:
        raise RuntimeError("expected one packaged raw crate")
    unpack = out / "unpacked"
    with tarfile.open(archives[0]) as archive:
        archive.extractall(unpack, filter="data")
    packaged = unpack / archives[0].name.removesuffix(".crate")
    manifest(packaged)
    run("resolve-package-lock", ["cargo", "generate-lockfile", "--manifest-path", str(fixture / "Cargo.toml"), *flags])
    run("unpacked-raw-consumer", cargo + ["--", "183", "9", "21"])
    results[-1].update(crate_sha256=digest(archives[0]),
                       consumer_lock_sha256=digest(fixture / "Cargo.lock"))
    (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"Native input acceptance complete: {out / 'results.json'}", flush=True)


if __name__ == "__main__":
    main()
