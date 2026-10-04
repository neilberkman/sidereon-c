#!/usr/bin/env python3
"""Check every C golden generator against the pinned sidereon engine source."""

from __future__ import annotations

import hashlib
import io
import json
import os
import re
import subprocess
import argparse
import tarfile
from pathlib import Path

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
GIT_PREFIX = "git+https://github.com/neilberkman/sidereon?rev="
VERSION = "3.0.1"
GENERATORS = ("fbgen", "velgen", "rinexgen", "sppgen", "pingen", "rtkgen", "valgen")


def fail(message: str) -> None:
    raise SystemExit(f"C generator source check: {message}")


def lock_packages(lock_path: Path) -> list[dict[str, object]]:
    try:
        text = lock_path.read_text()
    except OSError as error:
        fail(f"cannot read {lock_path}: {error}")
    rows = []
    for block in text.split("[[package]]")[1:]:
        row = {}
        for line in block.splitlines():
            key, separator, value = line.partition("=")
            if separator and key.strip() in {"name", "version", "source", "checksum"}:
                try:
                    row[key.strip()] = json.loads(value.strip())
                except json.JSONDecodeError:
                    fail(f"{lock_path}: malformed {key.strip()} field")
        rows.append(row)
    return rows


def manifest_identity(contents: bytes, path: Path) -> tuple[object, object]:
    try:
        text = contents.decode("utf-8")
    except UnicodeDecodeError as error:
        fail(f"{path} is not UTF-8: {error}")
    in_package = False
    fields = {}
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("[") and line.endswith("]"):
            in_package = line == "[package]"
        elif in_package:
            key, separator, value = line.partition("=")
            if separator and key.strip() in {"name", "version"}:
                try:
                    fields[key.strip()] = json.loads(value.strip())
                except json.JSONDecodeError:
                    fail(f"{path}: malformed {key.strip()} field")
    return fields.get("name"), fields.get("version")


def exact_locked(lock_path: Path, package: dict[str, object]) -> dict[str, object]:
    matches = [
        row
        for row in lock_packages(lock_path)
        if row.get("name") == package.get("name")
        and row.get("version") == package.get("version")
        and row.get("source") == package.get("source")
    ]
    if len(matches) != 1:
        fail(
            f"{lock_path}: expected one locked {package.get('name')} "
            f"{package.get('version')} from {package.get('source')!r}, found {len(matches)}"
        )
    return matches[0]


def cargo_metadata(manifest: Path) -> dict[str, object]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(manifest)],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        fail(f"cargo metadata --locked failed for {manifest}: {result.stderr.strip()}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        fail(f"cargo metadata returned invalid JSON for {manifest}: {error}")


def archive_vcs_revision(name: str, version: str, checksum: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{64}", checksum):
        fail(f"{name} {version} has a malformed or absent Cargo.lock checksum")
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cache_root = cargo_home / "registry" / "cache"
    if not cache_root.is_dir():
        fail(f"Cargo registry archive cache does not exist: {cache_root}")
    archive_name = f"{name}-{version}.crate"
    revisions: set[str] = set()
    for archive_path in cache_root.glob(f"*/{archive_name}"):
        archive = archive_path.read_bytes()
        if hashlib.sha256(archive).hexdigest() != checksum:
            continue
        prefix = f"{name}-{version}/"
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as bundle:
            wanted = {
                member.name[len(prefix) :]: member
                for member in bundle.getmembers()
                if member.name.startswith(prefix)
                and member.name[len(prefix) :] in {"Cargo.toml", ".cargo_vcs_info.json"}
            }
            if set(wanted) != {"Cargo.toml", ".cargo_vcs_info.json"}:
                fail(f"checksum-verified {archive_path} lacks manifest or VCS metadata")
            manifest_file = bundle.extractfile(wanted["Cargo.toml"])
            vcs_file = bundle.extractfile(wanted[".cargo_vcs_info.json"])
            if manifest_file is None or vcs_file is None:
                fail(f"cannot read source metadata from {archive_path}")
            manifest_name, manifest_version = manifest_identity(manifest_file.read(), archive_path)
            if manifest_name != name or manifest_version != version:
                fail(f"verified archive does not contain {name} {version}")
            try:
                vcs = json.loads(vcs_file.read())
            except json.JSONDecodeError as error:
                fail(f"invalid VCS metadata in verified {archive_path}: {error}")
            git = vcs.get("git")
            revision = git.get("sha1") if isinstance(git, dict) else None
            if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}", revision):
                fail(f"verified {name} {version} archive lacks a full source commit")
            if git.get("dirty") is True:
                fail(f"verified {name} {version} archive records a dirty source tree")
            revisions.add(revision)
    if len(revisions) != 1:
        fail(
            f"expected one checksum-matching archive source revision for {name} {version}; "
            f"found {sorted(revisions)}"
        )
    return next(iter(revisions))


def package_revision(package: dict[str, object], lock_path: Path, expected: str) -> tuple[str, str]:
    name = package.get("name")
    version = package.get("version")
    source = package.get("source")
    if name not in {"sidereon", "sidereon-core"}:
        fail(f"unexpected engine package {name!r}")
    if version != VERSION:
        fail(f"{name} must resolve to {VERSION}, got {version!r}")
    locked = exact_locked(lock_path, package)
    if isinstance(source, str) and source.startswith("git+"):
        if not source.startswith(GIT_PREFIX):
            fail(f"{name} uses a noncanonical Git source: {source}")
        value = source[len(GIT_PREFIX) :]
        requested, separator, resolved = value.partition("#")
        if not separator or not re.fullmatch(r"[0-9a-f]{40}", requested) or requested != resolved:
            fail(f"{name} source is not pinned to one full Git commit: {source}")
        if requested != expected:
            fail(f"{name} Git commit {requested} does not match CORE_REVISION {expected}")
        return "git", requested
    if source == REGISTRY:
        revision = archive_vcs_revision(name, version, str(locked.get("checksum", "")))
        if revision != expected:
            fail(f"{name} archive commit {revision} does not match CORE_REVISION {expected}")
        return "registry", revision
    fail(f"{name} has an unsupported Cargo source: {source!r}")


def graph_identity(manifest: Path, expected: str) -> tuple[str, str, str | None, list[dict[str, str]]]:
    metadata = cargo_metadata(manifest)
    packages = metadata.get("packages")
    resolution = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolution, dict):
        fail(f"cargo metadata omitted resolved packages for {manifest}")
    roots = [
        package
        for package in packages
        if Path(str(package.get("manifest_path", ""))).resolve() == manifest.resolve()
    ]
    if len(roots) != 1:
        fail(f"expected one root package for {manifest}, found {len(roots)}")
    root = roots[0]
    nodes = resolution.get("nodes")
    root_node = next((node for node in nodes if node.get("id") == root.get("id")), None)
    if root_node is None:
        fail(f"Cargo resolution has no root node for {manifest}")
    by_id = {package.get("id"): package for package in packages}
    for name in ("sidereon", "sidereon-core"):
        matches = [package for package in packages if package.get("name") == name]
        if len(matches) > 1:
            fail(f"{manifest}: ambiguous resolved {name} package identities")
    relevant = {name: [] for name in ("sidereon", "sidereon-core")}
    for dependency in root_node.get("deps", []):
        package = by_id.get(dependency.get("pkg"))
        if package and package.get("name") in relevant:
            relevant[package["name"]].append((dependency.get("name"), package))
    if len(relevant["sidereon-core"]) != 1:
        fail(f"{manifest}: expected one direct sidereon-core dependency")
    if root.get("name") == "sidereon-c" and len(relevant["sidereon"]) != 1:
        fail(f"{manifest}: expected one direct sidereon facade dependency")
    for name, entries in relevant.items():
        if len(entries) > 1:
            fail(f"{manifest}: ambiguous direct {name} dependencies")
        for alias, package in entries:
            if alias != name.replace("-", "_"):
                fail(f"{manifest}: {name} is renamed as dependency {alias!r}")
    core = relevant["sidereon-core"][0][1]
    workspace_root = metadata.get("workspace_root")
    if not isinstance(workspace_root, str) or not workspace_root:
        fail(f"cargo metadata omitted workspace_root for {manifest}")
    lock_path = Path(workspace_root) / "Cargo.lock"
    mode, revision = package_revision(core, lock_path, expected)
    engine_packages = [core]
    facades = [package for package in packages if package.get("name") == "sidereon"]
    if facades:
        facade_mode, facade_revision = package_revision(facades[0], lock_path, expected)
        if (facade_mode, facade_revision) != (mode, revision):
            fail(f"{manifest}: facade and core resolve to different source identities")
        engine_packages.append(facades[0])
    manifest_path = core.get("manifest_path")
    package_records = []
    for package in engine_packages:
        locked = exact_locked(lock_path, package)
        package_records.append(
            {
                "name": str(package["name"]),
                "version": str(package["version"]),
                "source": str(package["source"]),
                "checksum": str(locked["checksum"]) if locked.get("checksum") else "",
                "vcs_revision": revision,
            }
        )
    return (
        mode,
        revision,
        str(manifest_path) if isinstance(manifest_path, str) else None,
        package_records,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        action="append",
        default=[],
        help="also validate a locked standalone emitter manifest",
    )
    parser.add_argument(
        "--only-manifest",
        help="validate only this locked standalone manifest",
    )
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    binding_root = here.parent
    expected = (here / "CORE_REVISION").read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", expected):
        fail("CORE_REVISION must contain one full 40-character commit")
    if args.only_manifest:
        manifest = Path(args.only_manifest).resolve()
        mode, revision, core_manifest, packages = graph_identity(manifest, expected)
        print(
            json.dumps(
                {
                    "mode": mode,
                    "revision": revision,
                    "core_manifest": core_manifest,
                    "packages": packages,
                },
                sort_keys=True,
            )
        )
        return
    manifests = (
        [binding_root / "Cargo.toml"]
        + [here / name / "Cargo.toml" for name in GENERATORS]
        + [Path(value).resolve() for value in args.manifest]
    )
    identities = []
    core_manifest = None
    for manifest in manifests:
        mode, revision, resolved_core_manifest, packages = graph_identity(manifest, expected)
        identities.append((manifest, mode, revision, packages))
        if manifest == manifests[0]:
            core_manifest = resolved_core_manifest
    modes = {mode for _, mode, _, _ in identities}
    revisions = {revision for _, _, revision, _ in identities}
    if len(modes) != 1 or len(revisions) != 1:
        fail("binding root and all seven generators do not share one engine source identity")
    result = {
        "mode": next(iter(modes)),
        "revision": next(iter(revisions)),
        "core_manifest": core_manifest,
        "auxiliary_source": os.environ.get("SIDEREON_CORE_SOURCE"),
        "graphs": [
            {
                "manifest": (
                    str(manifest.relative_to(binding_root))
                    if manifest.is_relative_to(binding_root)
                    else manifest.name
                ),
                "mode": mode,
                "revision": revision,
                "packages": packages,
            }
            for manifest, mode, revision, packages in identities
        ],
    }
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
