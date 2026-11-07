#!/usr/bin/env python3
"""Measure full-sync and bounded-history updates with identical host output."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
module_spec = importlib.util.spec_from_file_location("workloads", ROOT / "scripts/check-ui-workloads.py")
w = importlib.util.module_from_spec(module_spec)
module_spec.loader.exec_module(w)


def parse(content):
    rows = [json.loads(line.split("HISTORY_JSON:", 1)[1]) for line in content.splitlines()
            if "HISTORY_JSON:" in line]
    meta = [json.loads(line.split("HISTORY_META:", 1)[1]) for line in content.splitlines()
            if "HISTORY_META:" in line]
    if len(rows) != 16 or len({w.key(row) for row in rows}) != 16 or len(meta) != 1 or "HISTORY_CORRECTNESS:PASS" not in content:
        raise RuntimeError("incomplete history output")
    return {"meta": meta[0], "rows": rows}


def stable(sample):
    omitted = set(w.PHASES) | {"native_high_water_since_init"}
    return [{k: v for k, v in row.items() if k not in omitted} for row in sample["rows"]]


def output(sample):
    fields = ("family", "scenario", "size", "series", "append_batch", "updates", "pixel_hash")
    return [{k: row[k] for k in fields} for row in sample["rows"]]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--locked", action="store_true")
    p.add_argument("--offline", action="store_true")
    p.add_argument("--artifacts", type=Path, required=True)
    p.add_argument("--correctness-only", action="store_true",
                        help="All cases and negative controls, one pair, no statistical acceptance")
    args = p.parse_args()
    pairs = 1 if args.correctness_only else w.PAIRS
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if (out / "results.json").exists():
        p.error("preserve prior results; choose a fresh artifact directory")
    for name in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        if os.environ.get(name):
            p.error("external override conflicts with fixed history configuration: " + name)
    if any(name.startswith("UI_HISTORY_") for name in os.environ):
        p.error("history controls are owned by this runner")
    host = subprocess.check_output(["rustc", "-vV"], text=True).split("host: ")[1].splitlines()[0]
    if os.environ.get("CARGO_BUILD_TARGET", host) != host:
        p.error("history execution requires the actual host target")
    before = w.inputs()
    config = ROOT / "rs_lvgl_ui/tests/leak-check/lv_conf.h"
    env = os.environ | {"LVGL_CONFIG_PATH": str(config), "CARGO_TARGET_DIR": str(out.parent / "history-build"),
                        "CARGO_TERM_COLOR": "never"}
    report = {"schema": 1, "passed": False, "source_before": before, "commands": []}

    def save():
        (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    def run(label, command, extra=None, expected=None):
        log = out / (label + ".log")
        start = time.time()
        with log.open("w") as stream:
            proc = subprocess.run(command, cwd=ROOT, env=env | (extra or {}),
                                  stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        content = log.read_text(errors="replace")
        passed = proc.returncode == 0 if expected is None else proc.returncode != 0 and expected in content
        report["commands"].append({"name": label, "command": command, "exit": proc.returncode,
                "passed": passed, "expected_failure": expected, "start_unix": start,
                "seconds": time.time() - start, "log_sha256": w.sha(log), "extra_environment": extra or {}})
        save()
        if not passed:
            raise RuntimeError(f"{label}: see {log}\n{content[-6000:]}")
        return content

    binaries, native = {}, {}
    for profile in ("debug", "release"):
        cmd = ["cargo", "test", "-p", "rs_lvgl_ui", "--test", "history_workloads", "--features", w.FEATURES,
               "--no-run", "--message-format=json"]
        cmd += (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
        if profile == "release":
            cmd += ["--release"]
        content = run("build-" + profile, cmd)
        events = [json.loads(line) for line in content.splitlines() if line.startswith("{")]
        found = [e["executable"] for e in events if e.get("reason") == "compiler-artifact"
                 and e.get("target", {}).get("name") == "history_workloads" and e.get("executable")]
        if len(found) != 1:
            raise RuntimeError("expected the actual history compiler artifact")
        binary = out / ("history-" + profile)
        shutil.copy2(found[0], binary)
        binaries[profile] = str(binary)
        outputs = [e["out_dir"] for e in events if e.get("reason") == "build-script-executed"
                   and "rs_lvgl_sys" in e["package_id"]]
        if not outputs:
            raise RuntimeError("missing native output provenance")
        native[profile] = [{"out_dir": path, "files": {
            q.relative_to(path).as_posix(): w.sha(q) for q in Path(path).rglob("*")
            if q.is_file() and q.suffix in (".a", ".h", ".rs")}} for path in outputs]
    report.update(binaries={k: {"path": v, "sha256": w.sha(Path(v))} for k, v in binaries.items()},
                  native_artifacts=native,
                  identity={"rustc": subprocess.check_output(["rustc", "-vV"], text=True).strip(),
                            "host": host, "features": w.FEATURES, "provider": "vendored LVGL 9.6.0",
                            "config_sha256": w.sha(config), "lock_sha256": w.sha(ROOT / "Cargo.lock"),
                            "fixture_sha256": w.sha(ROOT / "rs_lvgl_ui/tests/history_workloads.rs"),
                            "meter_sha256": w.sha(ROOT / "rs_lvgl_ui/tests/workloads/meter.rs")})
    invocation = ["bounded_history_workloads", "--exact", "--nocapture", "--test-threads=1"]
    allocations = {}
    for strategy in ("full", "append"):
        allocations[strategy] = parse(run("allocations-" + strategy, [binaries["debug"], *invocation],
                                          {"UI_HISTORY_STRATEGY": strategy}))
        if not allocations[strategy]["meta"]["allocation_instrumentation"]:
            raise RuntimeError("allocation sample lacks instrumentation")
        for corruption in ("samples", "pixels", "counts"):
            run("reject-" + strategy + "-" + corruption, [binaries["debug"], *invocation],
                {"UI_HISTORY_STRATEGY": strategy, "UI_HISTORY_CORRUPT": corruption},
                "HISTORY_ORACLE_" + corruption.upper())
    if output(allocations["full"]) != output(allocations["append"]):
        raise RuntimeError("debug full/history pixels differ")
    raw = {"A": [], "B": []}
    for pair in range(pairs):
        for side in (("A", "B") if pair % 2 == 0 else ("B", "A")):
            strategy = "full" if side == "A" else "append"
            sample = parse(run(f"timing-{pair:02}-{side}", [binaries["release"], *invocation],
                               {"UI_HISTORY_STRATEGY": strategy, "UI_HISTORY_CLOCKS": "on"}))
            if sample["meta"]["allocation_instrumentation"]:
                raise RuntimeError("allocation meter leaked into release timing")
            if raw[side] and stable(sample) != stable(raw[side][0]):
                raise RuntimeError("same-strategy output or counters changed")
            raw[side].append(sample)
        if output(raw["A"][-1]) != output(raw["B"][-1]):
            raise RuntimeError("paired full/history output differs")
        report["raw_samples"] = raw
        save()
        print(f"isolated history pair {pair + 1}/{pairs}: PASS", flush=True)
    overhead = {"on": [], "off": []}
    for index in range(4):
        for mode in (("on", "off") if index % 2 == 0 else ("off", "on")):
            sample = parse(run(f"clock-overhead-{index}-{mode}", [binaries["release"], *invocation],
                               {"UI_HISTORY_STRATEGY": "append", "UI_HISTORY_CLOCKS": mode}))
            if stable(sample) != stable(raw["B"][0]):
                raise RuntimeError("phase clocks changed history behavior")
            overhead[mode].append(sample)
    after = w.inputs()
    report.update(source_after=after, allocation_measurements=allocations,
                  raw_samples=raw, summary=None if args.correctness_only else {k: w.summarize(v) for k, v in raw.items()},
                  paired_ratios=None if args.correctness_only else w.paired(raw["A"], raw["B"]),
                  phase_clock_overhead={"raw_samples": overhead, "ratios": None if args.correctness_only else w.paired(overhead["off"], overhead["on"])},
                  protocol={"mode": "correctness-only" if args.correctness_only else "statistical-acceptance",
                            "statistical_acceptance": not args.correctness_only, "pairs": pairs, "order": "AB/BA alternating, one native process at a time",
                            "A": "full vectors", "B": "bounded history snapshots", "warmup": 4, "batch": 16},
                  limits={"snapshot": "bounded snapshot copying is included in view construction",
                          "update_excluding_construction": "validation/reconcile/runtime/drop, not pure diff",
                          "flush": "synchronous memory display; no device transfer",
                          "native_memory": "BUILTIN pool; high-water since initialization; allocation calls unavailable",
                          "timing": "host release; debug allocation meter separate; no board/FPS claim"})
    report["passed"] = before == after
    save()
    if not report["passed"]:
        raise RuntimeError("inputs changed during experiment")
    print(json.dumps({"passed": True, "cases_per_strategy": 16, "pairs": pairs, "results": str(out / "results.json")}, indent=2))


if __name__ == "__main__":
    main()
