#!/usr/bin/env python3
"""Build, test and run the native UI examples with bounded SDL lifetimes.

Requires Cargo, the native C/C++/bindgen toolchain and SDL2. On headless Linux,
run under xvfb-run. Pixel/interaction assertions use actual example views on
memory displays; SDL smoke is separate evidence and does not emulate OS input.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=Path("target/ui-audit"))
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = (root / args.artifacts).resolve()
    output.mkdir(parents=True, exist_ok=True)
    records = []
    env = os.environ.copy()
    env["LVGL_AUDIT_DIR"] = str(output / "images")
    # User-local screenshot/smoke settings must not alter these scenarios.
    for key in ("GALLERY_FRAMES", "COMPONENTS_FRAMES", "INPUT_FRAMES", "UI_FRAMES",
                "UI_AUTORUN", "DEMO_FRAMES", "DEMO_TAB", "DEMO_KEYBOARD",
                "DEMO_SCROLL_Y", "DEMO_SCREENSHOT", "STUDIO_FRAMES", "STUDIO_AUTORUN",
                "STUDIO_CAPTURE_DIR", "EXAMPLE_FRAMES", "EXAMPLE_CAPTURE_DIR", "EXAMPLE_DARK"):
        env.pop(key, None)

    def run(name, command, extra=None, timeout=600):
        print(f"[{name}] {' '.join(map(str, command))}", flush=True)
        start = time.time()
        log_path = output / f"{name}.log"
        code = 124
        with log_path.open("w") as log:
            try:
                result = subprocess.run(command, cwd=root, env=env | (extra or {}),
                                        stdout=log, stderr=subprocess.STDOUT, timeout=timeout)
                code = result.returncode
            except subprocess.TimeoutExpired:
                log.write(f"\nTimed out after {timeout}s\n")
        records.append({"name": name, "command": list(map(str, command)), "exit": code,
                        "start_unix": start, "duration_seconds": time.time() - start,
                        "scenario": extra or {},
                        "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest()})
        (output / "runs.json").write_text(json.dumps(records, indent=2) + "\n")
        if code != 0:
            print(log_path.read_text()[-12000:], file=sys.stderr)
            raise SystemExit(code if code > 0 else 1)

    flags = (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    features = ["--features", "showcase,font-cjk-14,rust_timer"]
    run("build", ["cargo", "build", "-p", "rs_lvgl_ui", *flags, "--examples", *features])
    run("tests", ["cargo", "test", "-p", "rs_lvgl_ui", *flags, "--examples", *features,
                  "--", "--test-threads=1"])
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version=1", "--no-deps", *flags], cwd=root, env=env))
    examples = Path(metadata["target_directory"]) / "debug" / "examples"
    scenarios = [
        ("settings", {}),
        ("studio", {"STUDIO_FRAMES": "460", "STUDIO_AUTORUN": "1"}),
        ("gallery", {"GALLERY_FRAMES": "80"}),
        ("components", {"COMPONENTS_FRAMES": "80"}),
        ("input", {"INPUT_FRAMES": "80"}),
        ("widgets_demo", {"DEMO_FRAMES": "80"}),
        ("widgets_demo", {"DEMO_FRAMES": "80", "DEMO_TAB": "analytics"}),
        ("widgets_demo", {"DEMO_FRAMES": "80", "DEMO_TAB": "shop"}),
        ("widgets_demo", {"DEMO_FRAMES": "80", "DEMO_KEYBOARD": "1"}),
        ("capabilities", {"UI_FRAMES": "900", "UI_AUTORUN": "1"}),
    ]
    for example in ("app", "demo", "bar", "button_click", "scale", "arc", "sdl",
                    "sdl_demo", "rust_timer", "animation"):
        scenarios.append((example, {"EXAMPLE_FRAMES": "100",
                                    "EXAMPLE_CAPTURE_DIR": str(output / "images")}))
    suffix = ".exe" if sys.platform == "win32" else ""
    for index, (example, settings) in enumerate(scenarios):
        binary = examples / (example + suffix)
        run(f"smoke-{index}-{example}", [str(binary)], settings, timeout=45)
    images = {str(p.relative_to(output)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted((output / "images").glob("*.ppm"))}
    (output / "images.json").write_text(json.dumps(images, indent=2) + "\n")
    print(f"Passed example regressions and {len(scenarios)} bounded native runs. Artifacts: {output}")


if __name__ == "__main__":
    main()
