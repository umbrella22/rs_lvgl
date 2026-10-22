#!/usr/bin/env python3
"""Render actual component-studio views to PNG using native LVGL memory displays.

Requires the Rust/native/SDL build dependencies, but no window server or Python
image package. Output images are converted from the fixture's lossless RGB PPMs.
Use --output docs/images to refresh the checked-in screenshots explicitly.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import time
import zlib

ROOT = Path(__file__).resolve().parents[1]
NAMES = ["overview", "controls", "input", "data", "layout", "media", "canvas", "composition", "native"]


def png_from_ppm(path):
    with path.open("rb") as stream:
        if stream.readline() != b"P6\n":
            raise ValueError(f"unexpected capture format: {path}")
        width, height = map(int, stream.readline().split())
        if stream.readline() != b"255\n":
            raise ValueError(f"unexpected channel depth: {path}")
        rgb = stream.read()
    if len(rgb) != width * height * 3 or (width, height) != (1280, 860):
        raise ValueError(f"truncated or incorrectly sized native capture: {path}")

    def chunk(kind, data):
        return struct.pack("!I", len(data)) + kind + data + struct.pack("!I", zlib.crc32(kind + data) & 0xffffffff)

    scanlines = b"".join(b"\0" + rgb[y * width * 3:(y + 1) * width * 3] for y in range(height))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack("!2I5B", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(scanlines, 9)) + chunk(b"IEND", b""))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/studio-audit")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--locked", action="store_true")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    artifacts = args.artifacts.resolve()
    output = args.output.resolve() if args.output else artifacts / "png"
    artifacts.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["LVGL_AUDIT_DIR"] = str(artifacts / "native")
    for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
        env.pop(key, None)
    flags = (["--locked"] if args.locked else []) + (["--offline"] if args.offline else [])
    command = ["cargo", "test", "-p", "rs_lvgl_ui", "--example", "studio", "--features", "showcase", *flags,
               "--", "--test-threads=1"]
    log = artifacts / "render.log"
    started = time.time()
    with log.open("w") as stream:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=600)
    if result.returncode:
        raise RuntimeError(f"native studio tests failed ({result.returncode}): {log}\n{log.read_text()[-4000:]}")
    images = {}
    for key, name in [(str(i), name) for i, name in enumerate(NAMES)] + [("dark", "dark")]:
        data = png_from_ppm(artifacts / "native" / f"studio-{key}.ppm")
        path = output / f"studio-{name}.png"
        path.write_bytes(data)
        images[path.name] = hashlib.sha256(data).hexdigest()
    # Portable receipt: source/artifact identities, no private paths or env dump.
    sources = [ROOT / "rs_lvgl_ui/examples/studio.rs", ROOT / "rs_lvgl_ui/examples/support/audit.rs",
               *sorted((ROOT / "rs_lvgl_ui/examples/studio").glob("*.rs")),
               ROOT / "rs_lvgl_ui/examples/assets/studio-motion.json", ROOT / "rs_lvgl_ui/examples/assets/status.gif"]
    receipt = {"renderer": "native LVGL 9.6 memory display", "dimensions": [1280, 860],
               "command": command, "exit": result.returncode, "start_unix": started,
               "duration_seconds": time.time() - started,
               "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
               "images_sha256": images, "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest()}
    (output / "studio-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Rendered {len(images)} actual studio views to {output}")


if __name__ == "__main__":
    main()
