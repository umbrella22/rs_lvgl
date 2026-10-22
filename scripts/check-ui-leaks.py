#!/usr/bin/env python3
"""Run strict Rust/LVGL heap accounting in an isolated native configuration."""

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
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/leak-audit")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--skip-allocator-control", action="store_true",
                        help="Run workloads only; not full acceptance")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    command = ["cargo", "test", "-p", "rs_lvgl_ui", "--test", "leak_check", "--features", "leak-check"]
    command += (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    env = os.environ.copy()
    env["CARGO_TERM_COLOR"] = "never"
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    config = ROOT / "rs_lvgl_ui/tests/leak-check/lv_conf.h"
    results = []

    def run(name, invocation, environment, expected=None):
        log = out / (name + ".log")
        start = time.time()
        with log.open("w") as stream:
            proc = subprocess.run(invocation, cwd=ROOT, env=environment,
                                  stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        content = log.read_text(errors="replace")
        passed = (proc.returncode == 0 if expected is None else
                  proc.returncode != 0 and expected in content)
        results.append(dict(name=name, command=invocation, exit=proc.returncode,
                            passed=passed, expected_failure=expected, seconds=time.time()-start,
                            start_unix=start, cwd=str(ROOT),
                            log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),
                            config_sha256=hashlib.sha256(config.read_bytes()).hexdigest() if expected is None else None))
        (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print(f"{name}: {'PASS' if passed else 'FAIL'}", flush=True)
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{content[-5000:]}")

    run("dual-heap", command + ["--", "--test-threads=1", "--nocapture"],
        env | {"LVGL_CONFIG_PATH": str(config), "CARGO_TARGET_DIR": str(out / "builtin")})
    if not args.skip_allocator_control:
        run("reject-unobservable-allocator", command + ["allocator_is_observable", "--", "--exact", "--nocapture", "--test-threads=1"],
            env | {"CARGO_TARGET_DIR": str(out / "clib")}, expected="LEAK_UNOBSERVABLE_NATIVE:")
    print(f"Leak measurements: {out / 'results.json'}", flush=True)


if __name__ == "__main__":
    main()
