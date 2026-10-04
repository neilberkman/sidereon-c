#!/usr/bin/env python3
"""Stage and run core fixture emitters without mutating any checkout."""

from __future__ import annotations

import io
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

EMITTERS = (
    ("tle_python_fixture", "iss_round_trip_fixture_self_validates"),
    ("sgp4_topocentric_arc", "iss_arc_walkers_match_single_instant_calls"),
    ("pass_finder_arc", "find_passes_agrees_with_reference_and_keeps_what_coarse_drops"),
    ("numerical_propagation", "reference_arc_is_deterministic_and_frozen"),
    ("sp3_bodies_python_fixture", "sp3_bodies_reference_self_validates"),
)
OUTPUTS = (
    "tle_roundtrip.json",
    "sgp4_topocentric.json",
    "pass_finder.json",
    "numerical_propagation.json",
    "sp3_bodies.json",
)
CANONICAL_ORIGINS = {
    "https://github.com/neilberkman/sidereon",
    "https://github.com/neilberkman/sidereon.git",
    "git@github.com:neilberkman/sidereon.git",
}


def fail(message: str) -> None:
    raise SystemExit(f"core fixture staging: {message}")


def git(source: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args], capture_output=True, text=True, check=False
    )
    if check and result.returncode:
        fail(f"git {' '.join(args)} failed in {source}: {result.stderr.strip()}")
    return result.stdout.strip()


def checked_source(report: dict[str, object], expected: str) -> tuple[Path, str, str]:
    mode = report["mode"]
    configured = os.environ.get("SIDEREON_CORE_SOURCE", "").strip() or None
    if configured:
        candidate = Path(configured).resolve()
        is_configured = True
    elif mode == "git" and report.get("core_manifest"):
        manifest = Path(str(report["core_manifest"]))
        candidate = Path(git(manifest.parent, "rev-parse", "--show-toplevel"))
        is_configured = False
    elif mode == "git":
        fail("git fixture staging requires core_manifest or SIDEREON_CORE_SOURCE at the verified auxiliary checkout")
    else:
        fail("registry generation requires SIDEREON_CORE_SOURCE at the verified auxiliary checkout")
    if not (candidate / "crates/sidereon-core/tests/fixtures").is_dir():
        fail(f"core fixture directory is missing under {candidate}")
    origin = git(candidate, "remote", "get-url", "origin")
    if origin not in CANONICAL_ORIGINS:
        if not is_configured:
            fail(
                f"auxiliary source has noncanonical origin {origin!r}; "
                "provide verified SIDEREON_CORE_SOURCE with a canonical checkout"
            )
        fail(f"auxiliary source has noncanonical origin {origin!r}")
    revision = git(candidate, "rev-parse", "HEAD")
    if revision != expected:
        fail(f"auxiliary source commit {revision} does not match verified engine commit {expected}")
    dirty = git(
        candidate,
        "status",
        "--porcelain",
        "--untracked-files=all",
        "--",
        "crates/sidereon-core/tests",
    )
    if dirty:
        fail("auxiliary core test sources or fixture data have local modifications")
    return candidate, origin, revision


def tree_sha256(paths: list[Path], relative_to: Path) -> str:
    digest = hashlib.sha256()
    files = sorted(
        file
        for path in paths
        for file in ([path] if path.is_file() else path.rglob("*"))
        if file.is_file()
    )
    for file in files:
        digest.update(file.relative_to(relative_to).as_posix().encode())
        digest.update(b"\0")
        digest.update(hashlib.sha256(file.read_bytes()).digest())
    return digest.hexdigest()


def safe_extract(archive: bytes, destination: Path) -> None:
    root = destination.resolve()
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as bundle:
        for member in bundle.getmembers():
            target = (destination / member.name).resolve()
            if target != root and root not in target.parents:
                fail(f"Git archive contains an unsafe path: {member.name!r}")
            if member.issym() or member.islnk():
                fail(f"Git archive contains an unexpected link: {member.name!r}")
        bundle.extractall(destination)


def git_archive(source: Path, destination: Path) -> None:
    raw = subprocess.run(
        ["git", "-C", str(source), "archive", "--format=tar", "HEAD"],
        capture_output=True,
        check=False,
    )
    if raw.returncode:
        fail(f"cannot stage candidate Git source: {raw.stderr.decode(errors='replace')}")
    destination.mkdir(parents=True, exist_ok=True)
    safe_extract(raw.stdout, destination)


def exact_test(command: list[str], env: dict[str, str], name: str, label: str | None = None) -> None:
    result = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
    output = result.stdout + result.stderr
    if result.returncode:
        fail(f"emitter {name} failed:\n{output}")
    if not re.search(rf"(?m)^test {re.escape(name)} \.\.\. ok$", output):
        fail(f"emitter {name} did not report its exact passing test:\n{output}")
    if not re.search(
        r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; [0-9]+ filtered out; finished in .+$",
        output,
    ):
        fail(f"emitter {name} did not prove a one-test successful run:\n{output}")
    label = label or name
    prefix = "registry-linked " if label.startswith("registry::") else ""
    print(f"PASS {prefix}core fixture emitter: {label.removeprefix('registry::')}", file=sys.stderr)


def candidate_emitters(source: Path, scratch: Path) -> Path:
    staged = scratch / "candidate-source"
    git_archive(source, staged)
    manifest = staged / "crates/sidereon-core/Cargo.toml"
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(scratch / "candidate-target")
    env["SIDEREON_DUMP_FIXTURES"] = "1"
    for target, test in EMITTERS:
        exact_test(
            [
                "cargo",
                "test",
                "--locked",
                "--manifest-path",
                str(manifest),
                "-p",
                "sidereon-core",
                "--test",
                target,
                test,
                "--",
                "--exact",
            ],
            env,
            test,
        )
    return staged / "bindings/python/tests/fixtures"


def copy_test_without_repo_gate(source: Path, target: Path) -> None:
    text = source.read_text()
    marker = "#![cfg(sidereon_repo_tests)]\n"
    if not text.startswith(marker):
        fail(f"unexpected leading crate gate in emitter source {source}")
    target.write_text(text[len(marker) :])


def registry_emitters(source: Path, scratch: Path, helper_root: Path) -> tuple[Path, dict[str, object]]:
    harness = scratch / "crates/sidereon-core"
    tests = harness / "tests"
    tests.mkdir(parents=True)
    (scratch / "bindings/python/tests/fixtures").mkdir(parents=True, exist_ok=True)
    template = helper_root / "registry_fixture_harness"
    manifest_source = template / "Cargo.toml"
    lock_source = template / "Cargo.lock"
    if not lock_source.is_file():
        fail(
            "registry-linked fixture harness has no resolved Cargo.lock yet; "
            "resolve its exact registry graph after sidereon 3.0.1 publication"
        )
    shutil.copy2(manifest_source, harness / "Cargo.toml")
    shutil.copy2(lock_source, harness / "Cargo.lock")
    fixture_source = source / "crates/sidereon-core/tests/fixtures"
    shutil.copytree(fixture_source, tests / "fixtures")
    for target, _test in EMITTERS:
        copy_test_without_repo_gate(
            source / "crates/sidereon-core/tests" / f"{target}.rs",
            tests / f"{target}.rs",
        )

    validator = helper_root / "validate_generator_sources.py"
    validation = subprocess.run(
        [sys.executable, str(validator), "--only-manifest", str(harness / "Cargo.toml")],
        capture_output=True,
        text=True,
        check=False,
    )
    if validation.returncode:
        fail(f"registry fixture harness source validation failed: {validation.stderr.strip()}")
    harness_identity = json.loads(validation.stdout)
    if harness_identity.get("mode") != "registry" or harness_identity.get("revision") != (helper_root / "CORE_REVISION").read_text().strip():
        fail("fixture harness does not resolve the verified registry core")

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(scratch / "registry-target")
    env["SIDEREON_DUMP_FIXTURES"] = "1"
    for target, test in EMITTERS:
        exact_test(
            [
                "cargo",
                "test",
                "--locked",
                "--manifest-path",
                str(harness / "Cargo.toml"),
                "--test",
                target,
                test,
                "--",
                "--exact",
            ],
            env,
            test,
            f"registry::{test}",
        )
    return scratch / "bindings/python/tests/fixtures", {
        "manifest_sha256": hashlib.sha256((harness / "Cargo.toml").read_bytes()).hexdigest(),
        "lock_sha256": hashlib.sha256((harness / "Cargo.lock").read_bytes()).hexdigest(),
        "graph": {
            "mode": harness_identity["mode"],
            "revision": harness_identity["revision"],
            "packages": harness_identity["packages"],
        },
        "executed_tests": [test for _, test in EMITTERS],
    }


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: prepare_core_fixtures.py <source-report.json> <scratch-dir>")
    report = json.loads(Path(sys.argv[1]).read_text())
    scratch = Path(sys.argv[2]).resolve()
    helper_root = Path(__file__).resolve().parent
    expected = (helper_root / "CORE_REVISION").read_text().strip()
    if report.get("revision") != expected:
        fail("generator source report does not match CORE_REVISION")
    source, origin, revision = checked_source(report, expected)
    harness = None
    if report.get("mode") == "git":
        output = candidate_emitters(source, scratch)
    elif report.get("mode") == "registry":
        output, harness = registry_emitters(source, scratch, helper_root)
    else:
        fail(f"unknown engine source mode {report.get('mode')!r}")
    for name in OUTPUTS:
        path = output / name
        if not path.is_file() or path.stat().st_size == 0:
            fail(f"named emitters did not write nonempty fixture {path}")
    fixture_root = source / "crates/sidereon-core/tests/fixtures"
    emitter_sources = [source / "crates/sidereon-core/tests" / f"{target}.rs" for target, _ in EMITTERS]
    context = {
        "schema_version": 1,
        "engine_source_mode": report["mode"],
        "engine_revision": revision,
        "cargo_graphs": report["graphs"],
        "auxiliary_fixture_source": {
            "origin": origin,
            "commit": revision,
            "fixtures_sha256": tree_sha256([fixture_root], source),
            "emitter_sources_sha256": tree_sha256(emitter_sources, source),
        },
        "emitted_core_fixtures_sha256": tree_sha256([output], scratch),
        "registry_emitter_harness": harness,
    }
    (scratch / "generation-context.json").write_text(json.dumps(context, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"core_fixtures": str(source / "crates/sidereon-core/tests/fixtures"), "emitted_fixtures": str(output)}))


if __name__ == "__main__":
    main()
