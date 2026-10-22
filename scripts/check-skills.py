#!/usr/bin/env python3
"""Verify portable skill installation, collision refusal and the included Rust example."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def files(directory):
    return {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(directory.rglob("*")) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/skill-audit")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--locked", action="store_true")
    args = parser.parse_args()
    out = args.artifacts.resolve()
    out.mkdir(parents=True, exist_ok=True)
    records = []
    env = os.environ.copy()
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    env["CARGO_TARGET_DIR"] = str(out / "build")

    def run(name, command, expected=None):
        start = time.time()
        log = out / (name + ".log")
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream,
                                    stderr=subprocess.STDOUT, timeout=600)
        text = log.read_text(errors="replace")
        passed = (result.returncode == 0 if expected is None else
                  result.returncode != 0 and expected in text)
        records.append(dict(name=name, command=command, start_unix=start,
                            seconds=time.time()-start, exit=result.returncode, passed=passed,
                            log_sha256=hashlib.sha256(log.read_bytes()).hexdigest()))
        (out / "results.json").write_text(json.dumps(records, indent=2) + "\n")
        if not passed:
            raise RuntimeError(f"{name}: see {log}\n{text[-2500:]}")
        print(f"{name}: PASS", flush=True)

    with tempfile.TemporaryDirectory(prefix="installed-skills-", dir=out) as temporary:
        install = Path(temporary) / "skills"
        command = [sys.executable, str(ROOT / "scripts/install-skills.py"), "--dest", str(install)]
        run("install", command)
        original = files(ROOT / ".agents/skills")
        assert files(install) == original, "installed contents differ"
        run("collision-refused", command, expected="destination exists; no files copied")
        assert files(install) == original, "collision modified installed skills"
        for markdown in install.rglob("*.md"):
            for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", markdown.read_text()):
                if "://" in target or target.startswith("#"):
                    continue
                assert (markdown.parent / target.split("#")[0]).is_file(), target
        fixture = out / "consumer"
        (fixture / "src/bin").mkdir(parents=True, exist_ok=True)
        manifest = fixture / "Cargo.toml"
        manifest.write_text('[package]\nname="skill-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[dependencies]\n'
            f'rs_lvgl={{path={json.dumps(str(ROOT / "rs_lvgl"))},default-features=false,features=["alloc"]}}\n'
            f'rs_lvgl_ui={{path={json.dumps(str(ROOT / "rs_lvgl_ui"))},default-features=false,features=["alloc"]}}\n')
        shutil.copyfile(install / "rs-lvgl-ui/references/counter.rs", fixture / "src/main.rs")
        snippet = re.findall(r"```rust\n(.*?)```", (ROOT / "README.md").read_text(), re.S)[0]
        translated = re.findall(r"```rust\n(.*?)```", (ROOT / "README.zh.md").read_text(), re.S)[0]
        assert snippet == translated, "Chinese and English README examples differ"
        (fixture / "src/bin/readme.rs").write_text(snippet +
            '\nfn main() { let mut counter = Counter { value: 0 }; counter.update(Msg::Increment); '
            'let _ = counter.view(&Theme::dark()); assert_eq!(counter.value, 1); }\n')
        flags = ["--offline"] if args.offline else []
        run("resolve-consumer", ["cargo", "generate-lockfile", "--manifest-path", str(manifest), *flags])
        flags += ["--locked"] if args.locked else []
        run("sample-lifecycle", ["cargo", "run", "--manifest-path", str(manifest), "--bin", "skill-consumer", *flags])
        run("readme-example", ["cargo", "run", "--manifest-path", str(manifest), "--bin", "readme", *flags])
    print("Copied skill contents, references, collision handling and examples passed.")


if __name__ == "__main__":
    main()
