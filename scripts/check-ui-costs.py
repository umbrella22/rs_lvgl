#!/usr/bin/env python3
"""Measure descriptor updates with a native BUILTIN pool and Rust allocation meter."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def source_identity():
    fingerprint_path = ROOT / "docs/plans/ui-reliability-fingerprint.py"
    if fingerprint_path.exists():
        spec = importlib.util.spec_from_file_location("fingerprint", fingerprint_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module.fingerprint()
    # Consumer/CI checkouts do not include maintainer-local plans.
    paths = subprocess.check_output(
        ["git", "ls-files", "-cz", "--others", "--exclude-standard"], cwd=ROOT).split(b"\0")
    digest = hashlib.sha256()
    for name in sorted(set(filter(None, paths))):
        path = ROOT / os.fsdecode(name)
        digest.update(name + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest() if path.is_file() else b"absent")
    return {"algorithm": "cost-source-files-v1", "sha256": digest.hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/ui-improvements/costs")
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--static-text", action="store_true")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    config = ROOT / "rs_lvgl_ui/tests/leak-check/lv_conf.h"
    env.update(LVGL_CONFIG_PATH=str(config), CARGO_TARGET_DIR=str(out.parent / "cost-build"), CARGO_TERM_COLOR="never")
    features = "leak-check,update-metrics" + (",cost-static-text" if args.static_text else "")
    cmd = ["cargo", "test", "-p", "rs_lvgl_ui", "--test", "update_cost", "--features", features]
    cmd += (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    cmd += ["--", "--test-threads=1", "--nocapture"]
    before = source_identity()
    start = time.time()
    log = out / "measurements.log"
    with log.open("w") as stream:
        result = subprocess.run(cmd, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=600)
    content = log.read_text()
    rows = [json.loads(line.split("COST_JSON:", 1)[1]) for line in content.splitlines() if "COST_JSON:" in line]
    after = source_identity()
    passed = result.returncode == 0 and "COST_CONTROL:PASS" in content and len(rows) == 3 and before == after
    identity = {"command": cmd, "exit": result.returncode, "passed": passed, "start_unix": start,
                "seconds": time.time()-start, "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                "target": subprocess.check_output(["rustc", "-vV"], text=True).split("host: ")[1].splitlines()[0],
                "profile": "test/unoptimized", "config_sha256": hashlib.sha256(config.read_bytes()).hexdigest(),
                "lock_sha256": hashlib.sha256((ROOT/"Cargo.lock").read_bytes()).hexdigest(),
                "fixture_sha256": hashlib.sha256((ROOT/"rs_lvgl_ui/tests/update_cost.rs").read_bytes()).hexdigest(),
                "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(), "measurements": rows,
                "source_before": before, "source_after": after,
                "limits": {"native_high_water": "since initialization, not per-update peak", "native_allocation_calls": "not instrumented", "native_setter_calls": "label text only", "timing": "host test profile only"}}
    (out/"results.json").write_text(json.dumps(identity, indent=2)+"\n")
    if not passed:
        raise RuntimeError(content[-5000:])
    print(json.dumps({"passed": passed, "measurements": rows}, indent=2))


if __name__ == "__main__":
    main()
