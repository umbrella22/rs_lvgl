#!/usr/bin/env python3
"""Verify, publish and reconcile the four rs_lvgl crates from one version tag."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import time
import tomllib
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ("rs_lvgl_codegen", "rs_lvgl_sys", "rs_lvgl", "rs_lvgl_ui")
API = "https://crates.io/api/v1/crates"
USER_AGENT = "cargo (rs_lvgl release; https://github.com/umbrella22/rs_lvgl)"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def release_version(requested=None):
    versions = set()
    for name in PACKAGES:
        manifest = tomllib.loads((ROOT / name / "Cargo.toml").read_text())
        package = manifest["package"]
        if package["name"] != name or package.get("publish") in (False, []):
            raise RuntimeError(f"{name} is not a publishable release package")
        versions.add(package["version"])
        for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
            for dependency, value in manifest.get(kind, {}).items():
                if dependency in PACKAGES and value.get("version") != package["version"]:
                    raise RuntimeError(f"{name}: internal dependency {dependency} has a different version")
    if len(versions) != 1:
        raise RuntimeError("All four package versions must agree")
    version = versions.pop()
    if not re.fullmatch(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)", version):
        raise RuntimeError("This release workflow accepts stable MAJOR.MINOR.PATCH versions")
    if requested and requested not in (version, f"v{version}"):
        raise RuntimeError(f"Expected {version} or v{version}, got {requested}")
    return version


def require_tag(version):
    if git("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("Publication requires a clean checkout")
    tag = f"v{version}"
    commit = git("rev-parse", "HEAD")
    if git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}") != commit:
        raise RuntimeError(f"HEAD must be the exact {tag} commit")
    remote_main = git("ls-remote", "--exit-code", "origin", "refs/heads/main").split()[0]
    if git("rev-parse", "origin/main") != remote_main:
        raise RuntimeError("origin/main is stale; update the main tracking ref before publication")
    remote_tags = git("ls-remote", "--exit-code", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}")
    refs = dict(line.split()[::-1] for line in remote_tags.splitlines())
    if refs.get(f"refs/tags/{tag}^{{}}", refs.get(f"refs/tags/{tag}")) != commit:
        raise RuntimeError("Remote release tag differs from HEAD")
    if subprocess.run(["git", "merge-base", "--is-ancestor", commit, "origin/main"],
                      cwd=ROOT, check=False).returncode:
        raise RuntimeError(f"{tag} must be reachable from origin/main")
    return commit


def registry_record(name, version):
    request = urllib.request.Request(f"{API}/{name}/{version}",
                                     headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            record = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        status = error.code
        error.close()
        if status == 404:
            return None
        raise RuntimeError(f"crates.io returned HTTP {status} for {name} {version}") from None
    if (record.get("crate") != name or record.get("num") != version or record.get("yanked")
            or not re.fullmatch(r"[0-9a-f]{64}", record.get("checksum", ""))):
        raise RuntimeError(f"Invalid or yanked crates.io record for {name} {version}")
    return record


def archive_files(data, name, version):
    prefix = f"{name}-{version}/"
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive.getmembers():
            if (not member.name.startswith(prefix) or not member.isfile()
                    or ".." in Path(member.name).parts):
                raise RuntimeError(f"Unexpected archive member: {member.name}")
            relative = member.name.removeprefix(prefix)
            if relative in files:
                raise RuntimeError(f"Duplicate archive member: {member.name}")
            files[relative] = archive.extractfile(member).read()
    for required in ("Cargo.toml", "Cargo.toml.orig", "README.md", "LICENSE", "src/lib.rs"):
        if required not in files:
            raise RuntimeError(f"{name} archive is missing {required}")
    if any(path.startswith(("docs/plans/", ".git/")) or path == "AGENTS.md" for path in files):
        raise RuntimeError(f"{name} archive includes internal repository documents")
    manifest = tomllib.loads(files["Cargo.toml"].decode())
    if manifest["package"]["name"] != name or manifest["package"]["version"] != version:
        raise RuntimeError(f"{name} archive identity does not match the release")
    for target in manifest.get("example", []):
        if target["path"] not in files:
            raise RuntimeError(f"{name} archive has a missing example: {target['path']}")
    return files


def reconcile_archive(local, remote, checksum, name, version, commit):
    if hashlib.sha256(remote).hexdigest() != checksum:
        raise RuntimeError(f"Registry download checksum mismatch for {name} {version}")
    expected = archive_files(local, name, version)
    published = archive_files(remote, name, version)
    for files in (expected, published):
        vcs = json.loads(files.get(".cargo_vcs_info.json", b"{}"))
        if vcs.get("git", {}).get("sha1") != commit or vcs.get("git", {}).get("dirty"):
            raise RuntimeError(f"{name} archive was not built from clean tag commit {commit}")
    # Cargo regenerates these two files. The registry's original lockfile is
    # retained; every other file must equal the freshly packaged tagged source.
    for generated in ("Cargo.lock", ".cargo_vcs_info.json"):
        expected.pop(generated, None)
        published.pop(generated, None)
    if expected != published:
        raise RuntimeError(f"Published source differs from the tag for {name} {version}")


class Release:
    def __init__(self, version, out):
        self.version = version
        self.out = out.resolve()
        self.out.mkdir(parents=True, exist_ok=True)
        self.env = os.environ.copy()
        self.env["CARGO_TERM_COLOR"] = "never"
        self.env["CARGO_TARGET_DIR"] = str(self.out / "build")
        for key in ("LVGL_CONFIG_PATH", "LVGL_EXTRA_C_DIR", "LVGL_EXTRA_HEADERS", "LVGL_CFLAGS"):
            self.env.pop(key, None)
        self.receipt = {"version": version, "commit": git("rev-parse", "HEAD"),
                        "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
                        "lock_sha256": None, "commands": [], "archives": {}, "registry": {}}
        lock = ROOT / "Cargo.lock"
        if lock.is_file():
            self.receipt["lock_sha256"] = hashlib.sha256(lock.read_bytes()).hexdigest()

    def save(self):
        (self.out / "release.json").write_text(json.dumps(self.receipt, indent=2) + "\n")

    def run(self, name, command, cwd=ROOT):
        log = self.out / f"{name}.log"
        started = time.time()
        print(f"{name}: running", flush=True)
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=cwd, env=self.env, stdout=stream,
                                    stderr=subprocess.STDOUT, timeout=1800, check=False)
        self.receipt["commands"].append({"name": name, "command": command,
            "cwd": str(cwd), "start_unix": started, "seconds": time.time() - started,
            "exit": result.returncode, "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest()})
        self.save()
        if result.returncode:
            raise RuntimeError(f"{name} failed; see {log}\n{log.read_text(errors='replace')[-2500:]}")
        print(f"{name}: PASS", flush=True)

    def package(self, allow_dirty=False):
        command = ["cargo", "package", "--workspace", "--registry", "crates-io", "--locked"]
        if allow_dirty:
            command.append("--allow-dirty")
        self.run("package", command)
        archives = {}
        for name in PACKAGES:
            filename = f"{name}-{self.version}.crate"
            candidates = (self.out / "build/package/tmp-crate" / filename,
                          self.out / "build/package" / filename)
            source = next((path for path in candidates if path.is_file()), None)
            if source is None:
                raise RuntimeError(f"Cargo did not create {filename}")
            data = source.read_bytes()
            files = archive_files(data, name, self.version)
            target = self.out / filename
            target.write_bytes(data)
            archives[name] = target
            self.receipt["archives"][name] = {"sha256": hashlib.sha256(data).hexdigest(),
                                              "files": len(files), "bytes": len(data)}
        self.save()
        return archives

    def consumers(self, archives=None):
        with tempfile.TemporaryDirectory(prefix="rs-lvgl-consumer-", dir=self.out) as temporary:
            fixture_root = Path(temporary)
            patches = {}
            if archives:
                for name, path in archives.items():
                    with tarfile.open(path) as archive:
                        archive.extractall(fixture_root, filter="data")
                    patches[name] = fixture_root / f"{name}-{self.version}"
            raw = '''fn main() {
    // SAFETY: this isolated process is the only native LVGL user; no callbacks,
    // aliases, render or asynchronous transfer survive display deletion.
    unsafe {
        rs_lvgl_sys::lv_init();
        let display = rs_lvgl_sys::lv_display_create(32, 32);
        assert!(!display.is_null());
        rs_lvgl_sys::lv_display_delete(display);
        rs_lvgl_sys::lv_deinit();
    }
    println!("REGISTRY_RAW_OK");
}
'''
            ui = (ROOT / ".agents/skills/rs-lvgl-ui/references/counter.rs").read_text()
            scenarios = {
                "raw": (f'rs_lvgl_sys = {{ version = "={self.version}", features = ["minimal-config"] }}\n', raw),
                "ui": (f'rs_lvgl = {{ version = "={self.version}", default-features = false, features = ["alloc"] }}\n'
                       f'rs_lvgl_ui = "={self.version}"\n', ui),
            }
            for scenario, (dependencies, source) in scenarios.items():
                fixture = fixture_root / scenario
                (fixture / "src").mkdir(parents=True)
                manifest = ('[package]\nname = "release-consumer-' + scenario
                            + '"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n[dependencies]\n'
                            + dependencies)
                if patches:
                    manifest += "\n[patch.crates-io]\n" + "".join(
                        f"{name} = {{ path = {json.dumps(str(path))} }}\n" for name, path in patches.items())
                (fixture / "Cargo.toml").write_text(manifest)
                (fixture / "src/main.rs").write_text(source)
                self.run(f"consumer-{scenario}", ["cargo", "run"], cwd=fixture)
                metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked",
                    "--format-version=1"], cwd=fixture, env=self.env, stderr=subprocess.DEVNULL))
                expected = set(PACKAGES) if scenario == "ui" else {"rs_lvgl_sys"}
                found = set()
                for package in metadata["packages"]:
                    if package["name"] not in expected:
                        continue
                    found.add(package["name"])
                    if package["version"] != self.version:
                        raise RuntimeError("Consumer resolved an unexpected package version")
                    if patches:
                        if Path(package["manifest_path"]).parent != patches[package["name"]]:
                            raise RuntimeError("Consumer did not use the extracted package")
                    elif package["source"] != "registry+https://github.com/rust-lang/crates.io-index":
                        raise RuntimeError("Registry consumer used a non-crates.io dependency")
                if found != expected:
                    raise RuntimeError(f"Consumer package graph is incomplete: {found}")
            self.receipt["consumer_source"] = "extracted archives" if archives else "crates.io only"
            self.save()

    def reconcile(self, name, archive, commit, record):
        url = f"https://static.crates.io/crates/{name}/{name}-{self.version}.crate"
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=60) as response:
            remote = response.read()
        reconcile_archive(archive.read_bytes(), remote, record["checksum"], name, self.version, commit)
        self.receipt["registry"][name] = {"checksum": record["checksum"], "source_commit": commit}
        self.save()
        print(f"{name}: registry checksum and tagged source verified", flush=True)

    def publish(self):
        commit = require_tag(self.version)
        archives = self.package()
        # Reconcile the complete existing set before performing any new upload.
        records = {name: registry_record(name, self.version) for name in PACKAGES}
        for name, record in records.items():
            if record:
                self.reconcile(name, archives[name], commit, record)
        for name in PACKAGES:
            if records[name]:
                continue
            require_tag(self.version)
            self.run(f"publish-{name}", ["cargo", "publish", "--registry", "crates-io",
                                        "--locked", "--package", name])
            for attempt in range(24):
                record = registry_record(name, self.version)
                if record:
                    break
                time.sleep(5)
            else:
                raise RuntimeError(f"Timed out waiting for {name} on crates.io; resume the same tag")
            self.reconcile(name, archives[name], commit, record)
        self.consumers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("verify", "publish", "status", "consumer", "identity"))
    parser.add_argument("version", nargs="?")
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/release-audit")
    parser.add_argument("--allow-dirty", action="store_true", help="Allow local changes for verify only")
    args = parser.parse_args()
    if args.allow_dirty and args.command != "verify":
        parser.error("--allow-dirty applies only to verify")
    version = release_version(args.version)
    if args.command == "identity":
        print(f"v{version}: {require_tag(version)}")
        return
    release = Release(version, args.artifacts)
    if args.command == "verify":
        release.consumers(release.package(args.allow_dirty))
    elif args.command == "publish":
        release.publish()
    elif args.command == "consumer":
        release.consumers()
    else:
        missing = []
        for name in PACKAGES:
            record = registry_record(name, version)
            print(f"{name} {version}: {'available' if record else 'missing'}")
            if not record:
                missing.append(name)
            else:
                release.receipt["registry"][name] = {"checksum": record["checksum"]}
        release.save()
        if missing:
            raise RuntimeError("Unpublished packages: " + ", ".join(missing))
    print(f"rs_lvgl {version}: {args.command} passed", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError, urllib.error.URLError) as error:
        raise SystemExit(str(error)) from None
