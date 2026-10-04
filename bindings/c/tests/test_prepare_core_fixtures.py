#!/usr/bin/env python3
"""Isolated checks for core fixture staging and emitter progress isolation."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("prepare_core_fixtures.py")
SPEC = importlib.util.spec_from_file_location("prepare_core_fixtures", SCRIPT)
PREPARE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(PREPARE)


class PrepareCoreFixturesRegressionTests(unittest.TestCase):
    def setUp(self) -> None:
        self._env_patch = patch.dict(os.environ, {}, clear=False)
        self._env_patch.start()
        os.environ.pop("SIDEREON_CORE_SOURCE", None)
        self.temporary = tempfile.TemporaryDirectory()
        self.temp_dir = Path(self.temporary.name).resolve()
        self.source_dir = self.temp_dir / "auxiliary_source"
        self.scratch_dir = self.temp_dir / "scratch"
        self.scratch_dir.mkdir(parents=True)
        self.revision = (SCRIPT.parent / "CORE_REVISION").read_text().strip()

        fixture_dir = self.source_dir / "crates/sidereon-core/tests/fixtures"
        fixture_dir.mkdir(parents=True)
        (fixture_dir / "seed.txt").write_text("seed fixture data\n")

        tests_dir = self.source_dir / "crates/sidereon-core/tests"
        for target, _ in PREPARE.EMITTERS:
            (tests_dir / f"{target}.rs").write_text("// emitter source\n")

        manifest = self.source_dir / "crates/sidereon-core/Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-core"\nversion = "3.0.1"\n')

        self.report_data = {
            "mode": "git",
            "revision": self.revision,
            "core_manifest": str(manifest),
            "graphs": [
                {
                    "manifest": "Cargo.toml",
                    "mode": "git",
                    "revision": self.revision,
                    "packages": [],
                }
            ],
        }
        self.report_path = self.temp_dir / "source-report.json"
        self.report_path.write_text(json.dumps(self.report_data))

    def tearDown(self) -> None:
        self.temporary.cleanup()
        self._env_patch.stop()

    def make_tar_bytes(self) -> bytes:
        tar_io = io.BytesIO()
        with tarfile.open(fileobj=tar_io, mode="w") as tf:
            manifest_data = b'[package]\nname = "sidereon-core"\nversion = "3.0.1"\n'
            ti = tarfile.TarInfo("crates/sidereon-core/Cargo.toml")
            ti.size = len(manifest_data)
            tf.addfile(ti, io.BytesIO(manifest_data))
        return tar_io.getvalue()

    def create_mock_subprocess_run(
        self,
        *,
        stage_outputs: bool = True,
        omitted_outputs: set[str] | None = None,
        empty_outputs: set[str] | None = None,
        cargo_returncode: int = 0,
        test_name_override: str | None = None,
        zero_tests: bool = False,
    ):
        omitted = omitted_outputs or set()
        empty = empty_outputs or set()

        def fake_subprocess_run(cmd, *args, **kwargs):
            if cmd[0] == "git":
                if "archive" in cmd:
                    return subprocess.CompletedProcess(
                        cmd, 0, stdout=self.make_tar_bytes(), stderr=b""
                    )
                if "--show-toplevel" in cmd:
                    return subprocess.CompletedProcess(
                        cmd, 0, stdout=f"{self.source_dir}\n", stderr=""
                    )
                if "remote" in cmd and "get-url" in cmd:
                    return subprocess.CompletedProcess(
                        cmd, 0, stdout="https://github.com/neilberkman/sidereon\n", stderr=""
                    )
                if "rev-parse" in cmd and "HEAD" in cmd:
                    return subprocess.CompletedProcess(
                        cmd, 0, stdout=f"{self.revision}\n", stderr=""
                    )
                if "status" in cmd:
                    return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

            if cmd[0] == "cargo" and len(cmd) > 1 and cmd[1] == "test":
                if stage_outputs:
                    out_dir = self.scratch_dir / "candidate-source/bindings/python/tests/fixtures"
                    out_dir.mkdir(parents=True, exist_ok=True)
                    for output_name in PREPARE.OUTPUTS:
                        if output_name in omitted:
                            continue
                        target_file = out_dir / output_name
                        if output_name in empty:
                            target_file.write_text("")
                        else:
                            target_file.write_text(f'{{"fixture": "{output_name}"}}\n')

                if cargo_returncode != 0:
                    return subprocess.CompletedProcess(
                        cmd, cargo_returncode, stdout="", stderr="error: test execution failed\n"
                    )

                test_name = cmd[9] if len(cmd) > 9 else "test"
                reported_name = test_name_override if test_name_override is not None else test_name
                passed_count = 0 if zero_tests else 1
                stdout = (
                    f"running 1 test\n"
                    f"test {reported_name} ... ok\n\n"
                    f"test result: ok. {passed_count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
                )
                return subprocess.CompletedProcess(cmd, 0, stdout=stdout, stderr="")

            return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

        return fake_subprocess_run

    def test_production_preparation_entry_point_pure_json_stdout_and_progress_stderr(self) -> None:
        fake_run = self.create_mock_subprocess_run(stage_outputs=True)
        captured_stdout = io.StringIO()
        captured_stderr = io.StringIO()

        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with contextlib.redirect_stdout(captured_stdout), contextlib.redirect_stderr(captured_stderr):
                    PREPARE.main()

        stdout_str = captured_stdout.getvalue()
        stderr_str = captured_stderr.getvalue()

        # Stdout must be strictly parseable as JSON without progress pollution
        parsed = json.loads(stdout_str)
        self.assertIsInstance(parsed, dict)
        self.assertIn("core_fixtures", parsed)
        self.assertIn("emitted_fixtures", parsed)
        self.assertTrue(parsed["core_fixtures"])
        self.assertTrue(parsed["emitted_fixtures"])
        self.assertEqual(stdout_str.strip().count("\n"), 0)
        self.assertNotIn("PASS", stdout_str)

        # Stderr must contain all named emitter passes
        for _, test_name in PREPARE.EMITTERS:
            self.assertIn(f"PASS core fixture emitter: {test_name}", stderr_str)

        # Generation context artifact must be produced
        context_file = self.scratch_dir / "generation-context.json"
        self.assertTrue(context_file.is_file())
        context = json.loads(context_file.read_text())
        self.assertEqual(context.get("engine_source_mode"), "git")
        self.assertEqual(context.get("engine_revision"), self.revision)

    def test_shell_handoff_json_parsing(self) -> None:
        fake_run = self.create_mock_subprocess_run(stage_outputs=True)
        fixture_paths_file = self.scratch_dir / "fixture-paths.json"

        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with open(fixture_paths_file, "w") as out_stream:
                    with contextlib.redirect_stdout(out_stream):
                        PREPARE.main()

        # Emulate run_generators.sh lines 25-26: json.load on fixture-paths.json
        with open(fixture_paths_file) as f:
            handoff = json.load(f)
        core_fixtures = handoff["core_fixtures"]
        emitted_fixtures = handoff["emitted_fixtures"]
        self.assertTrue(Path(core_fixtures).is_dir())
        self.assertTrue(Path(emitted_fixtures).is_dir())

    def test_rejects_missing_exact_test_names(self) -> None:
        fake_run = self.create_mock_subprocess_run(test_name_override="unexpected_test_name")
        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "did not report its exact passing test"):
                    PREPARE.main()

    def test_rejects_zero_executed_tests(self) -> None:
        fake_run = self.create_mock_subprocess_run(zero_tests=True)
        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "did not prove a one-test successful run"):
                    PREPARE.main()

    def test_rejects_failed_command(self) -> None:
        fake_run = self.create_mock_subprocess_run(cargo_returncode=101)
        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "failed"):
                    PREPARE.main()

    def test_rejects_absent_emitted_files(self) -> None:
        fake_run = self.create_mock_subprocess_run(omitted_outputs={"sp3_bodies.json"})
        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "named emitters did not write nonempty fixture"):
                    PREPARE.main()

    def test_rejects_empty_emitted_files(self) -> None:
        fake_run = self.create_mock_subprocess_run(empty_outputs={"tle_roundtrip.json"})
        with patch("sys.argv", ["prepare_core_fixtures.py", str(self.report_path), str(self.scratch_dir)]):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "named emitters did not write nonempty fixture"):
                    PREPARE.main()

    def _create_source_tree(self, name: str) -> Path:
        tree = (self.temp_dir / name).resolve()
        fixture_dir = tree / "crates/sidereon-core/tests/fixtures"
        fixture_dir.mkdir(parents=True, exist_ok=True)
        manifest = tree / "crates/sidereon-core/Cargo.toml"
        manifest.write_text('[package]\nname = "sidereon-core"\nversion = "3.0.1"\n')
        return tree

    def create_git_metadata_runner(
        self,
        *,
        toplevel_map: dict[str, str] | None = None,
        origin_map: dict[str, str] | None = None,
        revision_map: dict[str, str] | None = None,
        status_map: dict[str, str] | None = None,
        default_origin: str = "https://github.com/neilberkman/sidereon",
        default_revision: str | None = None,
        default_status: str = "",
    ):
        def _resolve_key(k: str) -> str:
            try:
                return str(Path(k).resolve())
            except Exception:
                return str(k)

        toplevels = {_resolve_key(k): _resolve_key(v) for k, v in (toplevel_map or {}).items()}
        origins = {_resolve_key(k): v for k, v in (origin_map or {}).items()}
        revisions = {_resolve_key(k): v for k, v in (revision_map or {}).items()}
        statuses = {_resolve_key(k): v for k, v in (status_map or {}).items()}
        expected_rev = default_revision or self.revision

        def fake_git_run(cmd, *args, **kwargs):
            if cmd[0] == "git":
                raw_source = str(cmd[2]) if len(cmd) > 2 else ""
                source = _resolve_key(raw_source)
                git_args = list(cmd[3:])
                if git_args == ["rev-parse", "--show-toplevel"]:
                    res = toplevels.get(source, toplevels.get(raw_source, source))
                    return subprocess.CompletedProcess(cmd, 0, stdout=f"{res}\n", stderr="")
                if git_args == ["remote", "get-url", "origin"]:
                    res = origins.get(source, origins.get(raw_source, default_origin))
                    return subprocess.CompletedProcess(cmd, 0, stdout=f"{res}\n", stderr="")
                if git_args == ["rev-parse", "HEAD"]:
                    res = revisions.get(source, revisions.get(raw_source, expected_rev))
                    return subprocess.CompletedProcess(cmd, 0, stdout=f"{res}\n", stderr="")
                if git_args and git_args[0] == "status":
                    res = statuses.get(source, statuses.get(raw_source, default_status))
                    return subprocess.CompletedProcess(cmd, 0, stdout=res, stderr="")
                return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")
            return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

        return fake_git_run

    def test_checked_source_configured_checkout_beats_cargo_cache_in_git_mode(self) -> None:
        cache_source = self._create_source_tree("cargo_cache_checkout")
        cache_manifest = cache_source / "crates/sidereon-core/Cargo.toml"
        configured_source = self._create_source_tree("configured_canonical_checkout")

        report = {
            "mode": "git",
            "revision": self.revision,
            "core_manifest": str(cache_manifest),
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            toplevel_map={str(cache_manifest.parent): str(cache_source)},
            origin_map={
                str(cache_source): "file:///cargo/cache/git/db/sidereon-mock",
                str(configured_source): "https://github.com/neilberkman/sidereon",
            },
            revision_map={
                str(cache_source): self.revision,
                str(configured_source): self.revision,
            },
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(configured_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                candidate, origin, revision = PREPARE.checked_source(report, self.revision)

        self.assertEqual(candidate, configured_source.resolve())
        self.assertEqual(origin, "https://github.com/neilberkman/sidereon")
        self.assertEqual(revision, self.revision)
        self.assertNotEqual(candidate, cache_source.resolve())

    def test_checked_source_registry_configured_still_works(self) -> None:
        configured_source = self._create_source_tree("registry_configured_source")
        report = {
            "mode": "registry",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(configured_source): "https://github.com/neilberkman/sidereon.git"},
            revision_map={str(configured_source): self.revision},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(configured_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                candidate, origin, revision = PREPARE.checked_source(report, self.revision)

        self.assertEqual(candidate, configured_source.resolve())
        self.assertEqual(origin, "https://github.com/neilberkman/sidereon.git")
        self.assertEqual(revision, self.revision)

    def test_checked_source_rejects_wrong_revision(self) -> None:
        configured_source = self._create_source_tree("wrong_revision_source")
        report = {
            "mode": "git",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(configured_source): "https://github.com/neilberkman/sidereon"},
            revision_map={str(configured_source): "0" * 40},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(configured_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "does not match verified engine commit"):
                    PREPARE.checked_source(report, self.revision)

    def test_checked_source_rejects_noncanonical_origin(self) -> None:
        configured_source = self._create_source_tree("noncanonical_origin_source")
        report = {
            "mode": "git",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(configured_source): "https://example.com/untrusted/sidereon.git"},
            revision_map={str(configured_source): self.revision},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(configured_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(SystemExit, "auxiliary source has noncanonical origin"):
                    PREPARE.checked_source(report, self.revision)

    def test_checked_source_rejects_dirty_relevant_inputs(self) -> None:
        configured_source = self._create_source_tree("dirty_inputs_source")
        report = {
            "mode": "git",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(configured_source): "https://github.com/neilberkman/sidereon"},
            revision_map={str(configured_source): self.revision},
            status_map={str(configured_source): " M crates/sidereon-core/tests/foo.rs\n"},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(configured_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(
                    SystemExit, "auxiliary core test sources or fixture data have local modifications"
                ):
                    PREPARE.checked_source(report, self.revision)

    def test_checked_source_absent_configured_with_noncanonical_cache_refuses_with_actionable_diagnostic(
        self,
    ) -> None:
        cache_source = self._create_source_tree("cargo_cache_checkout")
        cache_manifest = cache_source / "crates/sidereon-core/Cargo.toml"
        noncanonical_url = "file:///cargo/cache/git/db/sidereon-mock"

        report = {
            "mode": "git",
            "revision": self.revision,
            "core_manifest": str(cache_manifest),
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            toplevel_map={str(cache_manifest.parent): str(cache_source)},
            origin_map={str(cache_source): noncanonical_url},
            revision_map={str(cache_source): self.revision},
        )

        env_without_configured = {k: v for k, v in os.environ.items() if k != "SIDEREON_CORE_SOURCE"}
        with patch.dict(os.environ, env_without_configured, clear=True):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaises(SystemExit) as cm:
                    PREPARE.checked_source(report, self.revision)

        error_message = str(cm.exception)
        self.assertIn("noncanonical origin", error_message)
        self.assertIn("SIDEREON_CORE_SOURCE", error_message)
        self.assertIn(noncanonical_url, error_message)

    def test_checked_source_configured_symlink_resolves_and_rejects_dirty(self) -> None:
        target_source = self._create_source_tree("symlink_target_source")
        symlink_source = self.temp_dir / "symlink_source"
        symlink_source.symlink_to(target_source)

        report = {
            "mode": "git",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(symlink_source): "https://github.com/neilberkman/sidereon"},
            revision_map={str(symlink_source): self.revision},
            status_map={str(symlink_source): " M crates/sidereon-core/tests/fixtures/seed.txt\n"},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(symlink_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                with self.assertRaisesRegex(
                    SystemExit, "auxiliary core test sources or fixture data have local modifications"
                ):
                    PREPARE.checked_source(report, self.revision)

    def test_checked_source_configured_symlink_resolves_canonical_candidate(self) -> None:
        target_source = self._create_source_tree("symlink_target_canonical")
        symlink_source = self.temp_dir / "symlink_source_canonical"
        symlink_source.symlink_to(target_source)

        report = {
            "mode": "git",
            "revision": self.revision,
            "graphs": [],
        }

        fake_run = self.create_git_metadata_runner(
            origin_map={str(symlink_source): "https://github.com/neilberkman/sidereon.git"},
            revision_map={str(symlink_source): self.revision},
        )

        with patch.dict(os.environ, {"SIDEREON_CORE_SOURCE": str(symlink_source)}):
            with patch("subprocess.run", side_effect=fake_run):
                candidate, origin, revision = PREPARE.checked_source(report, self.revision)

        self.assertEqual(candidate, target_source.resolve())
        self.assertEqual(origin, "https://github.com/neilberkman/sidereon.git")
        self.assertEqual(revision, self.revision)


if __name__ == "__main__":
    unittest.main()
