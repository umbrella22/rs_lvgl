#!/usr/bin/env python3
"""Copy the repository's LVGL skills into an explicit destination, without overwrite."""

import argparse
from pathlib import Path
import shutil

SOURCE = Path(__file__).resolve().parents[1] / ".agents/skills"
NAMES = ("rs-lvgl-ui", "rs-lvgl-native")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dest", required=True, type=Path, help="e.g. ~/.agents/skills")
    parser.add_argument("--skill", choices=NAMES, action="append",
                        help="Install only this skill; may be repeated (default: both)")
    args = parser.parse_args()
    names = list(dict.fromkeys(args.skill or NAMES))
    destination = args.dest.expanduser().resolve()
    for name in names:
        if not (SOURCE / name / "SKILL.md").is_file():
            parser.error(f"missing source skill: {name}")
        target = destination / name
        if target.exists() or target.is_symlink():
            parser.error(f"destination exists; no files copied: {target}")
    destination.mkdir(parents=True, exist_ok=True)
    for name in names:
        # copytree also refuses a destination created after the preflight check.
        shutil.copytree(SOURCE / name, destination / name)
        print(f"Installed {name}: {destination / name}")


if __name__ == "__main__":
    main()
