#!/usr/bin/env python3
"""Isolated checks for checksum-bound C generator source provenance."""

import hashlib
import importlib.util
import io
import json
import os
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from test_prepare_core_fixtures import PrepareCoreFixturesRegressionTests  # noqa: F401

SCRIPT = Path(__file__).with_name("validate_generator_sources.py")
SPEC = importlib.util.spec_from_file_location("validate_generator_sources", SCRIPT)
SOURCE_CHECK = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(SOURCE_CHECK)


def crate_archive(vcs: dict[str, object], name: str = "sidereon-core") -> bytes:
    members = {
        f"{name}-3.0.2/Cargo.toml": (
            f'[package]\nname = "{name}"\nversion = "3.0.2"\n'
        ).encode(),
        f"{name}-3.0.2/.cargo_vcs_info.json": json.dumps(vcs).encode(),
    }
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w:gz") as archive:
        for path, data in members.items():
            info = tarfile.TarInfo(path)
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))
    return output.getvalue()


class RegistryArchiveIdentityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.cargo_home = Path(self.temporary.name)
        self.cache = self.cargo_home / "registry/cache/test-index"
        self.cache.mkdir(parents=True)
        self.revision = "2373644611953e1833c0deb94057acc34bcec973"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_archive(self, archive: bytes) -> str:
        (self.cache / "sidereon-core-3.0.2.crate").write_bytes(archive)
        return hashlib.sha256(archive).hexdigest()

    def resolve(self, checksum: str) -> str:
        with patch.dict(os.environ, {"CARGO_HOME": str(self.cargo_home)}):
            return SOURCE_CHECK.archive_vcs_revision("sidereon-core", "3.0.2", checksum)

    def test_reads_vcs_from_the_lock_checksum_verified_archive(self) -> None:
        archive = crate_archive({"git": {"sha1": self.revision}})
        checksum = self.write_archive(archive)
        extracted = (
            self.cargo_home
            / "registry/src/test-index/sidereon-core-3.0.2/.cargo_vcs_info.json"
        )
        extracted.parent.mkdir(parents=True)
        extracted.write_text(json.dumps({"git": {"sha1": "f" * 40}}))
        self.assertEqual(self.resolve(checksum), self.revision)

    def test_rejects_a_dirty_published_source_record(self) -> None:
        archive = crate_archive({"git": {"sha1": self.revision, "dirty": True}})
        checksum = self.write_archive(archive)
        with self.assertRaisesRegex(SystemExit, "dirty source tree"):
            self.resolve(checksum)

    def test_rejects_an_archive_that_does_not_match_the_lock_checksum(self) -> None:
        self.write_archive(crate_archive({"git": {"sha1": self.revision}}))
        with self.assertRaisesRegex(SystemExit, "expected one checksum-matching archive"):
            self.resolve("0" * 64)

    def test_rejects_a_renamed_direct_engine_dependency(self) -> None:
        root_id = "path+file:///binding#sidereon-c@3.0.2"
        core_id = "registry+https://github.com/rust-lang/crates.io-index#sidereon-core@3.0.2"
        facade_id = "registry+https://github.com/rust-lang/crates.io-index#sidereon@3.0.2"
        manifest = self.cargo_home / "Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-c"\nversion = "3.0.2"\n')
        metadata = {
            "packages": [
                {"id": root_id, "name": "sidereon-c", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": SOURCE_CHECK.REGISTRY,
                    "manifest_path": str(self.cargo_home / "core/Cargo.toml"),
                },
                {
                    "id": facade_id,
                    "name": "sidereon",
                    "version": "3.0.2",
                    "source": SOURCE_CHECK.REGISTRY,
                    "manifest_path": str(self.cargo_home / "facade/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "engine_core", "pkg": core_id},
                            {"name": "sidereon", "pkg": facade_id},
                        ],
                    }
                ]
            },
        }
        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            with self.assertRaisesRegex(SystemExit, "renamed as dependency"):
                SOURCE_CHECK.graph_identity(manifest, self.revision)


class WorkspaceLockResolutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.cargo_home = Path(self.temporary.name)
        self.cache = self.cargo_home / "registry/cache/test-index"
        self.cache.mkdir(parents=True)
        self.revision = "2373644611953e1833c0deb94057acc34bcec973"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_binding_manifest_below_workspace_root_resolves_workspace_lock(self) -> None:
        workspace_root = self.cargo_home / "workspace"
        binding_dir = workspace_root / "bindings/c"
        binding_dir.mkdir(parents=True)
        manifest = binding_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-c"\nversion = "3.0.2"\n')

        git_source = f"{SOURCE_CHECK.GIT_PREFIX}{self.revision}#{self.revision}"
        lock_path = workspace_root / "Cargo.lock"
        lock_path.write_text(
            f'version = 3\n\n'
            f'[[package]]\nname = "sidereon-core"\nversion = "3.0.2"\nsource = "{git_source}"\n\n'
            f'[[package]]\nname = "sidereon"\nversion = "3.0.2"\nsource = "{git_source}"\n'
        )

        self.assertFalse((binding_dir / "Cargo.lock").exists())

        root_id = "path+file:///binding#sidereon-c@3.0.2"
        core_id = f"{git_source}#sidereon-core@3.0.2"
        facade_id = f"{git_source}#sidereon@3.0.2"
        metadata = {
            "workspace_root": str(workspace_root),
            "packages": [
                {"id": root_id, "name": "sidereon-c", "version": "3.0.2", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon-core/Cargo.toml"),
                },
                {
                    "id": facade_id,
                    "name": "sidereon",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "sidereon_core", "pkg": core_id},
                            {"name": "sidereon", "pkg": facade_id},
                        ],
                    }
                ]
            },
        }

        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            mode, revision, core_manifest, packages = SOURCE_CHECK.graph_identity(manifest, self.revision)

        self.assertEqual(mode, "git")
        self.assertEqual(revision, self.revision)
        self.assertEqual(core_manifest, str(workspace_root / "crates/sidereon-core/Cargo.toml"))
        self.assertEqual(len(packages), 2)
        package_names = {p["name"] for p in packages}
        self.assertEqual(package_names, {"sidereon-core", "sidereon"})

    def test_standalone_manifest_resolves_own_lock(self) -> None:
        standalone_dir = self.cargo_home / "fbgen"
        standalone_dir.mkdir(parents=True)
        manifest = standalone_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "fbgen"\nversion = "0.0.0"\n')

        git_source = f"{SOURCE_CHECK.GIT_PREFIX}{self.revision}#{self.revision}"
        lock_path = standalone_dir / "Cargo.lock"
        lock_path.write_text(
            f'version = 3\n\n'
            f'[[package]]\nname = "sidereon-core"\nversion = "3.0.2"\nsource = "{git_source}"\n'
        )

        root_id = "path+file:///fbgen#fbgen@0.0.0"
        core_id = f"{git_source}#sidereon-core@3.0.2"
        metadata = {
            "workspace_root": str(standalone_dir),
            "packages": [
                {"id": root_id, "name": "fbgen", "version": "0.0.0", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(self.cargo_home / "core/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "sidereon_core", "pkg": core_id},
                        ],
                    }
                ]
            },
        }

        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            mode, revision, core_manifest, packages = SOURCE_CHECK.graph_identity(manifest, self.revision)

        self.assertEqual(mode, "git")
        self.assertEqual(revision, self.revision)
        self.assertEqual(len(packages), 1)
        self.assertEqual(packages[0]["name"], "sidereon-core")

    def test_missing_workspace_lock_fails(self) -> None:
        workspace_root = self.cargo_home / "missing_lock_ws"
        binding_dir = workspace_root / "bindings/c"
        binding_dir.mkdir(parents=True)
        manifest = binding_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-c"\nversion = "3.0.2"\n')

        git_source = f"{SOURCE_CHECK.GIT_PREFIX}{self.revision}#{self.revision}"
        root_id = "path+file:///binding#sidereon-c@3.0.2"
        core_id = f"{git_source}#sidereon-core@3.0.2"
        facade_id = f"{git_source}#sidereon@3.0.2"
        metadata = {
            "workspace_root": str(workspace_root),
            "packages": [
                {"id": root_id, "name": "sidereon-c", "version": "3.0.2", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon-core/Cargo.toml"),
                },
                {
                    "id": facade_id,
                    "name": "sidereon",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "sidereon_core", "pkg": core_id},
                            {"name": "sidereon", "pkg": facade_id},
                        ],
                    }
                ]
            },
        }

        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            with self.assertRaisesRegex(SystemExit, "cannot read"):
                SOURCE_CHECK.graph_identity(manifest, self.revision)

    def test_mismatched_locked_source_fails(self) -> None:
        workspace_root = self.cargo_home / "mismatched_source_ws"
        binding_dir = workspace_root / "bindings/c"
        binding_dir.mkdir(parents=True)
        manifest = binding_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-c"\nversion = "3.0.2"\n')

        git_source = f"{SOURCE_CHECK.GIT_PREFIX}{self.revision}#{self.revision}"
        different_source = f"{SOURCE_CHECK.GIT_PREFIX}{'0' * 40}#{'0' * 40}"
        lock_path = workspace_root / "Cargo.lock"
        lock_path.write_text(
            f'version = 3\n\n'
            f'[[package]]\nname = "sidereon-core"\nversion = "3.0.2"\nsource = "{different_source}"\n\n'
            f'[[package]]\nname = "sidereon"\nversion = "3.0.2"\nsource = "{git_source}"\n'
        )

        root_id = "path+file:///binding#sidereon-c@3.0.2"
        core_id = f"{git_source}#sidereon-core@3.0.2"
        facade_id = f"{git_source}#sidereon@3.0.2"
        metadata = {
            "workspace_root": str(workspace_root),
            "packages": [
                {"id": root_id, "name": "sidereon-c", "version": "3.0.2", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon-core/Cargo.toml"),
                },
                {
                    "id": facade_id,
                    "name": "sidereon",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(workspace_root / "crates/sidereon/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "sidereon_core", "pkg": core_id},
                            {"name": "sidereon", "pkg": facade_id},
                        ],
                    }
                ]
            },
        }

        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            with self.assertRaisesRegex(SystemExit, "expected one locked sidereon-core"):
                SOURCE_CHECK.graph_identity(manifest, self.revision)

    def test_mismatched_locked_checksum_fails(self) -> None:
        workspace_root = self.cargo_home / "mismatched_checksum_ws"
        binding_dir = workspace_root / "bindings/c"
        binding_dir.mkdir(parents=True)
        manifest = binding_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-c"\nversion = "3.0.2"\n')

        archive = crate_archive({"git": {"sha1": self.revision}})
        (self.cache / "sidereon-core-3.0.2.crate").write_bytes(archive)
        (self.cache / "sidereon-3.0.2.crate").write_bytes(
            crate_archive({"git": {"sha1": self.revision}}, name="sidereon")
        )

        lock_path = workspace_root / "Cargo.lock"
        lock_path.write_text(
            f'version = 3\n\n'
            f'[[package]]\nname = "sidereon-core"\nversion = "3.0.2"\n'
            f'source = "{SOURCE_CHECK.REGISTRY}"\nchecksum = "{"0" * 64}"\n\n'
            f'[[package]]\nname = "sidereon"\nversion = "3.0.2"\n'
            f'source = "{SOURCE_CHECK.REGISTRY}"\nchecksum = "{"0" * 64}"\n'
        )

        root_id = "path+file:///binding#sidereon-c@3.0.2"
        core_id = f"{SOURCE_CHECK.REGISTRY}#sidereon-core@3.0.2"
        facade_id = f"{SOURCE_CHECK.REGISTRY}#sidereon@3.0.2"
        metadata = {
            "workspace_root": str(workspace_root),
            "packages": [
                {"id": root_id, "name": "sidereon-c", "version": "3.0.2", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": SOURCE_CHECK.REGISTRY,
                    "manifest_path": str(workspace_root / "core/Cargo.toml"),
                },
                {
                    "id": facade_id,
                    "name": "sidereon",
                    "version": "3.0.2",
                    "source": SOURCE_CHECK.REGISTRY,
                    "manifest_path": str(workspace_root / "facade/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": root_id,
                        "deps": [
                            {"name": "sidereon_core", "pkg": core_id},
                            {"name": "sidereon", "pkg": facade_id},
                        ],
                    }
                ]
            },
        }

        with patch.dict(os.environ, {"CARGO_HOME": str(self.cargo_home)}):
            with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
                with self.assertRaisesRegex(SystemExit, "expected one checksum-matching archive"):
                    SOURCE_CHECK.graph_identity(manifest, self.revision)

    def test_omitted_workspace_root_fails(self) -> None:
        standalone_dir = self.cargo_home / "no_ws_root"
        standalone_dir.mkdir(parents=True)
        manifest = standalone_dir / "Cargo.toml"
        manifest.write_text('[package]\nname = "fbgen"\nversion = "0.0.0"\n')

        git_source = f"{SOURCE_CHECK.GIT_PREFIX}{self.revision}#{self.revision}"
        root_id = "path+file:///fbgen#fbgen@0.0.0"
        core_id = f"{git_source}#sidereon-core@3.0.2"
        metadata = {
            "packages": [
                {"id": root_id, "name": "fbgen", "version": "0.0.0", "manifest_path": str(manifest)},
                {
                    "id": core_id,
                    "name": "sidereon-core",
                    "version": "3.0.2",
                    "source": git_source,
                    "manifest_path": str(self.cargo_home / "core/Cargo.toml"),
                },
            ],
            "resolve": {
                "nodes": [{"id": root_id, "deps": [{"name": "sidereon_core", "pkg": core_id}]}]
            },
        }

        with patch.object(SOURCE_CHECK, "cargo_metadata", return_value=metadata):
            with self.assertRaisesRegex(SystemExit, "cargo metadata omitted workspace_root"):
                SOURCE_CHECK.graph_identity(manifest, self.revision)


if __name__ == "__main__":
    unittest.main()
