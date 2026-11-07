#!/usr/bin/env python3
"""Verify and measure fixed host UI workloads; statistical runs are explicit acceptance."""
import argparse
import hashlib
import difflib
import json
import math
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
FEATURES = "leak-check,update-metrics"
PAIRS = 20
PHASES = ("construction_ns", "update_excluding_construction_ns", "memory_render_flush_ns", "update_total_ns")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inputs():
    names = set(filter(None, subprocess.check_output(
        ["git", "ls-files", "-cz", "--others", "--exclude-standard"], cwd=ROOT).decode().split("\0")))
    for path in (ROOT / "docs/plans").rglob("*"):
        if path.is_file() and "__pycache__" not in path.parts:
            names.add(path.relative_to(ROOT).as_posix())
    names.discard("docs/plans/ui-capability-depth-execution-ledger.md")
    if (ROOT / "Cargo.lock").exists():
        names.add("Cargo.lock")
    rows = {name: sha(ROOT / name) if (ROOT / name).is_file() else "absent" for name in sorted(names)}
    return {"algorithm": "workload-inputs-v1", "files": rows,
            "sha256": hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()}


def key(row):
    return ":".join(str(row[name]) for name in ("family", "scenario", "size", "series", "append_batch"))


def parse(content):
    rows = [json.loads(line.split("WORKLOAD_JSON:", 1)[1]) for line in content.splitlines()
            if "WORKLOAD_JSON:" in line]
    meta = [json.loads(line.split("WORKLOAD_META:", 1)[1]) for line in content.splitlines()
            if "WORKLOAD_META:" in line]
    if len(rows) != 53 or len(meta) != 1 or "WORKLOAD_CORRECTNESS:PASS" not in content:
        raise RuntimeError("incomplete workload output")
    if len({key(row) for row in rows}) != 53:
        raise RuntimeError("duplicate/missing workload identity")
    return {"meta": meta[0], "rows": rows}


def percentile(values, q):
    return sorted(values)[max(0, math.ceil(q * len(values)) - 1)]


def interval(values):
    rng = random.Random(6101)
    boot = [statistics.median(rng.choices(values, k=len(values))) for _ in range(1000)]
    return [percentile(boot, .025), percentile(boot, .975)]


def correctness(sample):
    # High-water is a since-init measurement, not a correctness oracle.
    timing = set(PHASES) | {"native_high_water_since_init"}
    return [{k: v for k, v in row.items() if k not in timing} for row in sample["rows"]]


def summarize(samples):
    result = {}
    for index, row in enumerate(samples[0]["rows"]):
        result[key(row)] = {phase: {"median_ns": statistics.median([s["rows"][index][phase] for s in samples]),
                                  "p95_ns": percentile([s["rows"][index][phase] for s in samples], .95)}
                            for phase in PHASES}
    return result


def paired(a, b):
    result = {}
    for index, row in enumerate(a[0]["rows"]):
        result[key(row)] = {}
        for phase in PHASES:
            ratios = [right["rows"][index][phase] / left["rows"][index][phase]
                      for left, right in zip(a, b) if left["rows"][index][phase] > 0]
            result[key(row)][phase] = {"b_over_a_median": statistics.median(ratios) if ratios else None,
                                      "median_bootstrap_95_interval": interval(ratios) if ratios else None,
                                      "raw_ratios": ratios}
    return result



def comparison_identity(baseline, identity, baseline_path, port_path=None):
    if baseline.get("protocol", {}).get("pairs", 0) < PAIRS:
        raise RuntimeError("baseline lacks the full statistical protocol")
    if not baseline.get("passed"):
        raise RuntimeError("baseline did not pass")
    differences = {name for name in set(baseline["identity"]) | set(identity)
                   if baseline["identity"].get(name) != identity.get(name)}
    if not differences:
        if port_path:
            raise RuntimeError("fixture-port manifest supplied without a fixture change")
        return {"baseline_sha256": sha(baseline_path), "fixture_port": None}
    if differences != {"fixture_sha256"} or not port_path:
        raise RuntimeError("baseline inputs/profile/fixture are not comparable")
    port = json.loads(port_path.read_text())
    if port.get("schema") != 1 or port.get("scope") != "chart constructor and owned series API spelling only":
        raise RuntimeError("unreviewed fixture-port scope")
    if port.get("baseline_results_sha256") != sha(baseline_path):
        raise RuntimeError("fixture-port baseline receipt changed")
    sources = {}
    for label, digest in (("original", baseline["identity"]["fixture_sha256"]),
                          ("ported", identity["fixture_sha256"])):
        path = (port_path.parent / port[label]["path"]).resolve()
        if sha(path) != digest or port[label]["sha256"] != digest:
            raise RuntimeError("fixture-port source identity changed: " + label)
        sources[label] = path.read_text()
    current = ROOT / "rs_lvgl_ui/tests/update_workloads.rs"
    if sources["ported"] != current.read_text():
        raise RuntimeError("current workload differs from reviewed port")
    diff = "".join(difflib.unified_diff(sources["original"].splitlines(True),
                    sources["ported"].splitlines(True), fromfile="original", tofile="ported"))
    diff_path = port_path.parent / port["diff"]["path"]
    if diff_path.read_text() != diff or sha(diff_path) != port["diff"]["sha256"]:
        raise RuntimeError("fixture-port diff differs from reviewed source")
    # Keep all preparation, timing boundaries, scenario loops and oracles byte
    # identical. The reviewed edit must be confined to the chart expression.
    start = "            Model::Chart {\n                series, history, ..\n            } => "
    end = "\n            .width(px(WIDTH as i16))"
    def outside(source):
        prefix, expression = source.split(start, 1)
        expression, suffix = expression.split(end, 1)
        return prefix, suffix
    if outside(sources["original"]) != outside(sources["ported"]):
        raise RuntimeError("fixture port changed workload semantics outside chart construction")
    return {"baseline_sha256": sha(baseline_path),
            "fixture_port": {"manifest_sha256": sha(port_path), "review": port, "exact_diff": diff}}


def semantic_output(sample):
    # Allocation/native memory and setter costs may change across source versions.
    # Scenario policy, application work, structural operations and pixels may not.
    fields = ("family", "scenario", "size", "series", "append_batch", "updates",
              "view_calls", "actions", "creates", "deletes", "moves", "pixel_hash")
    return [{name: row[name] for name in fields} for row in sample["rows"]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--correctness-only", action="store_true",
                        help="All cases and negative controls, one pair, no statistical acceptance")
    parser.add_argument("--compare", type=Path)
    parser.add_argument("--fixture-port", type=Path, help="Explicit reviewed API-only fixture port manifest")
    args = parser.parse_args()
    pairs = 1 if args.correctness_only else PAIRS
    if args.correctness_only and (args.compare or args.fixture_port):
        parser.error("--correctness-only cannot compare performance or use a fixture port")
    if args.fixture_port and not args.compare:
        parser.error("--fixture-port requires --compare")
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if (out / "results.json").exists():
        parser.error("preserve the prior result; choose a fresh artifact directory")
    for name in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        if os.environ.get(name):
            parser.error("external native override conflicts with the fixed workload configuration: " + name)
    if any(name.startswith("UI_WORKLOAD_") for name in os.environ):
        parser.error("workload control inputs are owned by this runner")
    host = subprocess.check_output(["rustc", "-vV"], text=True).split("host: ")[1].splitlines()[0]
    if os.environ.get("CARGO_BUILD_TARGET", host) != host:
        parser.error("workload execution requires the actual host target")
    before = inputs()
    config = ROOT / "rs_lvgl_ui/tests/leak-check/lv_conf.h"
    build_dir = out.parent / "workload-build"
    env = os.environ | {"LVGL_CONFIG_PATH": str(config), "CARGO_TARGET_DIR": str(build_dir),
                        "CARGO_TERM_COLOR": "never"}
    records = []
    report = {"schema": 1, "passed": False, "source_before": before, "commands": records}

    def save():
        (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    def run(label, command, extra=None, expected=None):
        log = out / (label + ".log")
        started = time.time()
        with log.open("w") as stream:
            proc = subprocess.run(command, cwd=ROOT, env=env | (extra or {}),
                                  stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        content = log.read_text(errors="replace")
        passed = proc.returncode == 0 if expected is None else proc.returncode != 0 and expected in content
        records.append({"name": label, "command": command, "exit": proc.returncode, "passed": passed,
                        "expected_failure": expected, "start_unix": started, "seconds": time.time() - started,
                        "log_sha256": sha(log), "extra_environment": extra or {}})
        save()
        if not passed:
            raise RuntimeError(f"{label}: see {log}\n{content[-6000:]}")
        return content

    binaries = {}
    native = {}
    for profile in ("debug", "release"):
        cmd = ["cargo", "test", "-p", "rs_lvgl_ui", "--test", "update_workloads",
               "--features", FEATURES, "--no-run", "--message-format=json"]
        cmd += (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
        if profile == "release":
            cmd += ["--release"]
        content = run("build-" + profile, cmd)
        events = []
        for line in content.splitlines():
            if line.startswith("{"):
                events.append(json.loads(line))
        found = [e["executable"] for e in events if e.get("reason") == "compiler-artifact"
                 and e.get("target", {}).get("name") == "update_workloads" and e.get("executable")]
        if len(found) != 1:
            raise RuntimeError("expected the actual workload compiler artifact")
        destination = out / ("workloads-" + profile)
        shutil.copy2(found[0], destination)
        binaries[profile] = str(destination)
        outputs = [e["out_dir"] for e in events if e.get("reason") == "build-script-executed"
                   and "rs_lvgl_sys" in e["package_id"]]
        if not outputs:
            raise RuntimeError("missing native build output provenance")
        native[profile] = [{"out_dir": path, "files": {
            p.relative_to(path).as_posix(): sha(p) for p in Path(path).rglob("*")
            if p.is_file() and p.suffix in (".a", ".h", ".rs")}} for path in outputs]
    report.update(binaries={profile: {"path": path, "sha256": sha(Path(path))}
                            for profile, path in binaries.items()}, native_artifacts=native)
    invocation = ["representative_workloads", "--exact", "--nocapture", "--test-threads=1"]
    allocation = parse(run("allocation-debug", [binaries["debug"], *invocation]))
    for family in ("settings", "list", "chart", "pixels"):
        run("reject-corruption-" + family, [binaries["debug"], *invocation],
            {"UI_WORKLOAD_CORRUPT": family}, "WORKLOAD_ORACLE_" + family.upper())
    report["allocation_measurements"] = allocation
    identity = {"rustc": subprocess.check_output(["rustc", "-vV"], text=True).strip(),
                "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
                "compiler": subprocess.check_output(["clang", "--version"], text=True).splitlines()[:2],
                "features": FEATURES, "profile": "release/System allocator; debug/Meter separate",
                "provider": "vendored LVGL 9.6.0", "allocator": "native BUILTIN 8MiB",
                "lock_sha256": sha(ROOT / "Cargo.lock"), "config_sha256": sha(config),
                "base_config_sha256": sha(ROOT / "rs_lvgl_sys/vendor/include-9.6/lv_conf.h"),
                "build_environment_sha256": {name: hashlib.sha256(value.encode()).hexdigest()
                    for name, value in sorted(os.environ.items())
                    if name in ("CC", "CXX", "AR", "CFLAGS", "CXXFLAGS", "RUSTFLAGS", "LIBCLANG_PATH",
                                "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET")
                    or name.startswith(("LVGL_", "CARGO_TARGET_", "BINDGEN_EXTRA_CLANG_ARGS", "CC_", "CXX_"))},
                "native_source_sha256": hashlib.sha256(json.dumps({name: digest
                    for name, digest in before["files"].items()
                    if name.startswith(("rs_lvgl_sys/", "rs_lvgl_codegen/", "rs_lvgl/"))},
                    sort_keys=True).encode()).hexdigest(),
                "fixture_sha256": sha(ROOT / "rs_lvgl_ui/tests/update_workloads.rs"),
                "meter_sha256": sha(ROOT / "rs_lvgl_ui/tests/workloads/meter.rs")}
    report["identity"] = identity
    baseline = None
    if args.compare:
        baseline = json.loads(args.compare.read_text())
        comparison = comparison_identity(baseline, identity, args.compare, args.fixture_port)
        old_binary = Path(baseline["binaries"]["release"]["path"])
        if sha(old_binary) != baseline["binaries"]["release"]["sha256"]:
            raise RuntimeError("baseline binary identity changed")
        report["comparison"] = comparison | {"purpose": "explicit paired source comparison"}
        a_binary = str(old_binary)
    else:
        a_binary = binaries["release"]
    raw = {"A": [], "B": []}
    for pair in range(pairs):
        order = ("A", "B") if pair % 2 == 0 else ("B", "A")
        for side in order:
            sample = parse(run(f"timing-{pair:02}-{side}",
                               [a_binary if side == "A" else binaries["release"], *invocation],
                               {"UI_WORKLOAD_CLOCKS": "on"}))
            if sample["meta"]["allocation_instrumentation"]:
                raise RuntimeError("allocation meter leaked into timing profile")
            if raw[side] and correctness(sample) != correctness(raw[side][0]):
                raise RuntimeError("same-binary output/counters differ")
            raw[side].append(sample)
        if semantic_output(raw["A"][-1]) != semantic_output(raw["B"][-1]):
            raise RuntimeError("cross-source scenario/application/pixel output differs")
        if not baseline and correctness(raw["A"][-1]) != correctness(raw["B"][-1]):
            raise RuntimeError("same-binary paired output differs")
        report["raw_samples"] = raw
        save()
        print(f"isolated pair {pair+1}/{pairs}: PASS", flush=True)
    # Same executable, phase clocks on/off. Only total update time can be
    # compared here; off deliberately cannot separate construction.
    overhead = {"on": [], "off": []}
    for index in range(4):
        for mode in (("on", "off") if index % 2 == 0 else ("off", "on")):
            sample = parse(run(f"clock-overhead-{index}-{mode}", [binaries["release"], *invocation],
                               {"UI_WORKLOAD_CLOCKS": mode}))
            if correctness(sample) != correctness(raw["B"][0]):
                raise RuntimeError("phase clock instrumentation changed behavior")
            overhead[mode].append(sample)
    after = inputs()
    report.update(source_after=after, raw_samples=raw,
                  summary=None if args.correctness_only else {side: summarize(samples) for side, samples in raw.items()},
                  paired_ratios=None if args.correctness_only else paired(raw["A"], raw["B"]),
                  phase_clock_overhead={"raw_samples": overhead, "ratios": None if args.correctness_only else paired(overhead["off"], overhead["on"])},
                  protocol={"mode": "correctness-only" if args.correctness_only else "statistical-acceptance",
                            "statistical_acceptance": not args.correctness_only, "pairs": pairs, "order": "AB/BA alternating, one native process at a time",
                            "warmup": 4, "batch": 16, "clock": "std::time::Instant monotonic wall clock",
                            "cpu_policy": "OS-managed; no CPU pinning/frequency claim"},
                  limits={"update_excluding_construction": "validation/reconcile plus runtime/drop overhead; no pure-diff claim",
                          "native_setters": "label text only; all other native setter counts unavailable in original fixture",
                          "native_pool": "per-update high-water/allocation-call instrumentation unavailable",
                          "list_scrollbar": "explicit Off; viewport offsets and focus are independently asserted",
                          "chart_append_full_writes": "unavailable in original API; new adapter measured separately in W2",
                          "native_high_water": "since initialization, not per-update peak",
                          "native_allocation_calls": "unavailable", "flush": "synchronous memory display/no device transfer",
                          "timing": "host release only; allocation instrumentation is debug only"})
    report["passed"] = before == after
    save()
    if not report["passed"]:
        raise RuntimeError("inputs changed during the experiment")
    print(json.dumps({"passed": True, "scenarios": 53, "pairs": pairs,
                      "results": str(out / "results.json")}, indent=2))


if __name__ == "__main__":
    main()
