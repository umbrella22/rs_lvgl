"""Release regressions: immutable source, registry failures and upload barriers."""

import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

SPEC = importlib.util.spec_from_file_location("release", Path(__file__).resolve().parents[1] / "release.py")
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)
NAME = "rs_lvgl_ui"
VERSION = "0.1.0"
COMMIT = "a" * 40


def archive_bytes(source=b"pub fn counter() {}", lock=b"original lock", commit=COMMIT, dirty=False):
    files = {"Cargo.toml": f'[package]\nname="{NAME}"\nversion="{VERSION}"\n'.encode(),
             "Cargo.toml.orig": b"original manifest", "README.md": b"readme", "LICENSE": b"MIT",
             "src/lib.rs": source, "Cargo.lock": lock,
             ".cargo_vcs_info.json": json.dumps({"git": {"sha1": commit, "dirty": dirty}}).encode()}
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w:gz") as archive:
        for path, value in files.items():
            info = tarfile.TarInfo(f"{NAME}-{VERSION}/{path}")
            info.size = len(value)
            archive.addfile(info, io.BytesIO(value))
    return output.getvalue()


def reconcile(local, remote):
    release.reconcile_archive(local, remote, hashlib.sha256(remote).hexdigest(), NAME, VERSION, COMMIT)


class ArchiveTests(unittest.TestCase):
    def test_same_tag_allows_regenerated_lock_and_retains_original_source(self):
        reconcile(archive_bytes(lock=b"new resolution"), archive_bytes())

    def test_changed_source_cannot_be_silently_skipped(self):
        with self.assertRaisesRegex(RuntimeError, "Published source differs"):
            reconcile(archive_bytes(), archive_bytes(source=b"pub fn different() {}"))

    def test_wrong_or_dirty_commit_cannot_be_reused(self):
        for data in (archive_bytes(commit="b" * 40), archive_bytes(dirty=True)):
            with self.subTest(data_hash=hashlib.sha256(data).hexdigest()):
                with self.assertRaisesRegex(RuntimeError, "clean tag commit"):
                    reconcile(archive_bytes(), data)

    def test_corrupt_download_is_rejected_before_source_comparison(self):
        with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
            release.reconcile_archive(archive_bytes(), archive_bytes(), "0" * 64, NAME, VERSION, COMMIT)

    def test_wrong_package_identity_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "Unexpected archive member"):
            release.archive_files(archive_bytes(), "rs_lvgl_sys", VERSION)


class RegistryTests(unittest.TestCase):
    def test_only_404_means_unpublished(self):
        for status in (403, 429, 500):
            with self.subTest(status=status):
                with patch.object(release.urllib.request, "urlopen", side_effect=
                                  urllib.error.HTTPError("https://crates.io", status, "failure", {}, None)):
                    with self.assertRaisesRegex(RuntimeError, f"HTTP {status}"):
                        release.registry_record(NAME, VERSION)
        with patch.object(release.urllib.request, "urlopen", side_effect=
                          urllib.error.HTTPError("https://crates.io", 404, "missing", {}, None)):
            self.assertIsNone(release.registry_record(NAME, VERSION))

    def test_yanked_wrong_version_and_missing_checksum_are_rejected(self):
        valid = {"crate": NAME, "num": VERSION, "yanked": False, "checksum": "a" * 64}
        for changes in ({"yanked": True}, {"num": "0.2.0"}, {"checksum": ""}):
            response = io.BytesIO(json.dumps({"version": valid | changes}).encode())
            with patch.object(release.urllib.request, "urlopen", return_value=response):
                with self.assertRaisesRegex(RuntimeError, "Invalid or yanked"):
                    release.registry_record(NAME, VERSION)


class PublicationBarrierTests(unittest.TestCase):
    def setUp(self):
        self.release = object.__new__(release.Release)
        self.release.version = VERSION
        self.archives = {name: Path(f"{name}.crate") for name in release.PACKAGES}

    def test_conflicting_last_crate_blocks_every_new_upload(self):
        records = [None, None, None, {"checksum": "a" * 64}]
        with patch.object(release, "require_tag", return_value=COMMIT), \
                patch.object(self.release, "package", return_value=self.archives), \
                patch.object(release, "registry_record", side_effect=records), \
                patch.object(self.release, "reconcile", side_effect=RuntimeError("conflicting source")), \
                patch.object(self.release, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "conflicting source"):
                self.release.publish()
            run.assert_not_called()

    def test_partial_release_publishes_only_missing_dependents_in_order(self):
        record = {"checksum": "a" * 64}
        with patch.object(release, "require_tag", return_value=COMMIT), \
                patch.object(self.release, "package", return_value=self.archives), \
                patch.object(release, "registry_record", side_effect=[record, record, None, None, record, record]), \
                patch.object(self.release, "reconcile"), \
                patch.object(self.release, "run") as run, \
                patch.object(self.release, "consumers") as consumers:
            self.release.publish()
            self.assertEqual([call.args[1][-1] for call in run.call_args_list], ["rs_lvgl", "rs_lvgl_ui"])
            consumers.assert_called_once_with()

    def test_complete_release_reconciles_without_any_upload(self):
        with patch.object(release, "require_tag", return_value=COMMIT), \
                patch.object(self.release, "package", return_value=self.archives), \
                patch.object(release, "registry_record", return_value={"checksum": "a" * 64}), \
                patch.object(self.release, "reconcile") as reconcile, \
                patch.object(self.release, "run") as run, \
                patch.object(self.release, "consumers") as consumers:
            self.release.publish()
            self.assertEqual(reconcile.call_count, 4)
            run.assert_not_called()
            consumers.assert_called_once_with()


class TagTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="rs-lvgl-tag-test-")
        self.addCleanup(self.temporary.cleanup)
        base = Path(self.temporary.name)
        self.repo = base / "work"
        self.repo.mkdir()
        self.command("init", "-b", "main")
        self.command("config", "user.email", "test@example.invalid")
        self.command("config", "user.name", "Release test")
        (self.repo / "source.txt").write_text("release source")
        self.command("add", "source.txt")
        self.command("commit", "-m", "fixture")
        subprocess.run(["git", "init", "--bare", str(base / "origin.git")],
                       check=True, capture_output=True)
        self.command("remote", "add", "origin", str(base / "origin.git"))
        self.command("push", "-u", "origin", "main")
        self.command("tag", "-a", "v0.1.0", "-m", "fixture release")
        self.command("push", "origin", "v0.1.0")
        self.root_patch = patch.object(release, "ROOT", self.repo)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def command(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.repo, stderr=subprocess.DEVNULL,
                                       text=True).strip()

    def test_clean_main_and_detached_exact_tag_are_accepted(self):
        expected = self.command("rev-parse", "HEAD")
        self.assertEqual(release.require_tag(VERSION), expected)
        self.command("checkout", "--detach", "v0.1.0")
        self.assertEqual(release.require_tag(VERSION), expected)

    def test_dirty_source_is_rejected(self):
        (self.repo / "source.txt").write_text("uncommitted source")
        with self.assertRaisesRegex(RuntimeError, "clean checkout"):
            release.require_tag(VERSION)

    def test_new_main_source_cannot_publish_an_old_tag(self):
        (self.repo / "source.txt").write_text("new source")
        self.command("commit", "-am", "later source")
        with self.assertRaisesRegex(RuntimeError, "exact v0.1.0 commit"):
            release.require_tag(VERSION)

    def test_unpushed_tag_is_rejected(self):
        self.command("tag", "-a", "v0.2.0", "-m", "not pushed")
        with self.assertRaises(subprocess.CalledProcessError):
            release.require_tag("0.2.0")


if __name__ == "__main__":
    unittest.main()
