#!/usr/bin/env python3
"""Package all four crates, then build/run a consumer using only extracted sources."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATES = ("rs_lvgl_codegen", "rs_lvgl_sys", "rs_lvgl", "rs_lvgl_ui")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if (out / "results.json").exists():
        parser.error("preserve prior receipts; choose a fresh artifact directory")
    host = subprocess.check_output(["rustc", "-vV"], text=True).split("host: ")[1].splitlines()[0]
    if os.environ.get("CARGO_BUILD_TARGET", host) != host:
        parser.error("package runtime requires the actual host target")
    for name in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        if os.environ.get(name):
            parser.error("package acceptance owns native inputs: " + name)
    root_lock_before = sha(ROOT / "Cargo.lock")
    env = os.environ | {"CARGO_TERM_COLOR": "never", "CARGO_TARGET_DIR": str(out / "package-build")}
    flags = (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    report = {"schema": 1, "passed": False, "commands": [], "host": host,
              "root_lock_sha256": root_lock_before}

    def save():
        (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    def run(name, command, cwd=ROOT, overrides=None, marker=None, capture_stdout=False):
        log = out / (name + ".log")
        start = time.time()
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=cwd, env=env | (overrides or {}),
                                    stdout=subprocess.PIPE if capture_stdout else stream,
                                    stderr=stream if capture_stdout else subprocess.STDOUT,
                                    text=True, timeout=900)
            if capture_stdout:
                stream.write(result.stdout)
        content = log.read_text(errors="replace")
        passed = result.returncode == 0 and (marker is None or marker in content)
        report["commands"].append({"name": name, "command": command, "cwd": str(cwd),
                "exit": result.returncode, "passed": passed, "expected_marker": marker,
                "start_unix": start, "seconds": time.time() - start, "log_sha256": sha(log)})
        save()
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{content[-6000:]}")
        return result.stdout if capture_stdout else content

    run("package-workspace", ["cargo", "package", "--workspace", "--allow-dirty", "--no-verify", "--exclude-lockfile", *flags])
    extracted = {}
    archives = {}
    for name in CRATES:
        version = tomllib.loads((ROOT / name / "Cargo.toml").read_text())["package"]["version"]
        archive = out / "package-build/package" / f"{name}-{version}.crate"
        if not archive.is_file():
            raise RuntimeError("missing workspace archive: " + name)
        unpack = out / "unpacked"
        with tarfile.open(archive) as stream:
            stream.extractall(unpack, filter="data")
        directory = unpack / f"{name}-{version}"
        extracted[name] = directory
        archives[name] = {"path": str(archive), "sha256": sha(archive),
                          "contents": {p.relative_to(directory).as_posix(): sha(p)
                                       for p in directory.rglob("*") if p.is_file()}}
        package = tomllib.loads((directory / "Cargo.toml").read_text())
        for section in ("dependencies", "build-dependencies", "dev-dependencies"):
            for dependency in package.get(section, {}).values():
                if isinstance(dependency, dict) and "path" in dependency:
                    raise RuntimeError("packaged manifest retains a checkout path")
    consumer = out / "consumer"
    (consumer / "src").mkdir(parents=True)
    patch = "\n".join(f'{name}={{path={json.dumps(str(directory))}}}' for name, directory in extracted.items())
    ui_version = tomllib.loads((extracted["rs_lvgl_ui"] / "Cargo.toml").read_text())["package"]["version"]
    manifest = consumer / "Cargo.toml"
    manifest.write_text('[package]\nname="unpacked-ui-consumer"\nversion="0.0.0"\nedition="2021"\n'
        '[workspace]\n[dependencies]\n'
        f'rs_lvgl_ui={{version="={ui_version}",default-features=false,features=["alloc","widget-slider","widget-textarea","widget-chart","widget-arc"]}}\n'
        f'rs_lvgl={{version="={ui_version}",default-features=false,features=["alloc"]}}\n'
        f'rs_lvgl_sys={{version="={ui_version}"}}\n'
        '[patch.crates-io]\n' + patch + '\n')
    source = ROOT / ".agents/skills/rs-lvgl-ui/references/capability-depth.rs"
    (consumer / "src/main.rs").write_bytes(source.read_bytes())
    # Native configuration comes from the extracted provider, with no checkout
    # include or path dependency. All build output is fresh and isolated here.
    build = out / "consumer-build"
    native_env = {"CARGO_TARGET_DIR": str(build)}
    resolve_flags = ["--offline"] if args.offline else []
    run("resolve-consumer", ["cargo", "generate-lockfile", *resolve_flags], cwd=consumer, overrides=native_env)
    # Cargo may download target-specific packages to stderr during metadata.
    # Retain those diagnostics in the log while parsing only its JSON stdout.
    metadata = json.loads(run("consumer-metadata", ["cargo", "metadata", "--format-version=1", *flags],
                              cwd=consumer, overrides=native_env, capture_stdout=True))
    resolved = {}
    for name in CRATES:
        matches = [p for p in metadata["packages"] if p["name"] == name]
        if len(matches) != 1 or matches[0]["source"] is not None:
            raise RuntimeError("consumer did not resolve one extracted source: " + name)
        actual = Path(matches[0]["manifest_path"]).resolve()
        if actual != extracted[name] / "Cargo.toml":
            raise RuntimeError("source-checkout or registry fallback: " + name)
        resolved[name] = {"manifest": str(actual), "sha256": sha(actual), "package_id": matches[0]["id"]}
    content = run("build-consumer", ["cargo", "build", "--message-format=json", *flags], cwd=consumer, overrides=native_env)
    events = [json.loads(line) for line in content.splitlines() if line.startswith("{")]
    binaries = [e["executable"] for e in events if e.get("reason") == "compiler-artifact"
                and e.get("target", {}).get("name") == "unpacked-ui-consumer" and e.get("executable")]
    native = [e["out_dir"] for e in events if e.get("reason") == "build-script-executed" and "rs_lvgl_sys" in e["package_id"]]
    if len(binaries) != 1 or not native or not all(Path(p).is_relative_to(build) for p in native):
        raise RuntimeError("missing isolated executable/native build provenance")
    run("native-consumer", [binaries[0]], cwd=consumer, overrides=native_env, marker="CAPABILITY_DEPTH:PASS")
    report.update(archives=archives, resolved_packages=resolved,
                  consumer={"manifest_sha256": sha(manifest), "source_sha256": sha(consumer / "src/main.rs"),
                            "lock_sha256": sha(consumer / "Cargo.lock"), "binary_sha256": sha(Path(binaries[0]))},
                  native_artifacts=[{"out_dir": directory, "files": {
                      q.relative_to(directory).as_posix(): sha(q) for q in Path(directory).rglob("*")
                      if q.is_file() and q.suffix in (".a", ".h", ".rs")}} for directory in native],
                  root_lock_after_sha256=sha(ROOT / "Cargo.lock"),
                  limits="Local host package/native execution only; no registry publication, board or external-provider acceptance")
    report["passed"] = report["root_lock_after_sha256"] == root_lock_before
    save()
    if not report["passed"]:
        raise RuntimeError("root lockfile changed")
    print(json.dumps({"passed": True, "crates": list(CRATES), "results": str(out / "results.json")}, indent=2))


if __name__ == "__main__":
    main()
