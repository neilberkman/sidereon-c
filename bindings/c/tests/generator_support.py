"""Shared fixture paths for the C fixture generators and transcribers."""

import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))

with open(os.path.join(HERE, "CORE_REVISION")) as fh:
    CORE_REVISION = fh.read().strip()


def core_crate_dir():
    """Directory of the sidereon-core crate resolved by locked pingen metadata."""
    metadata = json.loads(
        subprocess.check_output(
            [
                "cargo",
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--quiet",
                "--manifest-path",
                os.path.join(HERE, "pingen", "Cargo.toml"),
            ],
        )
    )
    package_ids = {package["id"]: package for package in metadata["packages"]}
    root_id = metadata["resolve"]["root"]
    root_node = next(node for node in metadata["resolve"]["nodes"] if node["id"] == root_id)
    core_edges = [
        dependency
        for dependency in root_node["deps"]
        if package_ids[dependency["pkg"]]["name"] == "sidereon-core"
    ]
    if len(core_edges) != 1 or core_edges[0]["name"] != "sidereon_core":
        raise RuntimeError("pingen must directly resolve one canonical sidereon-core dependency")
    manifest = package_ids[core_edges[0]["pkg"]]["manifest_path"]
    return os.path.dirname(manifest)


def core_fixtures_dir():
    """Verified auxiliary fixtures, or the locked Git package's test fixtures."""
    configured = os.environ.get("SIDEREON_CORE_FIXTURES")
    if configured:
        return os.path.abspath(configured)
    return os.path.join(core_crate_dir(), "tests", "fixtures")


def revision_comment():
    """The C comment line every generated header starts with."""
    return f"/* sidereon-core revision {CORE_REVISION} */"


def emitted_fixtures_dir():
    """Where sidereon-core's fixture emitters write.

    tests/run_generators.sh stages these files outside the checkout and passes
    the location through SIDEREON_EMITTED_FIXTURES.
    """
    configured = os.environ.get("SIDEREON_EMITTED_FIXTURES")
    if configured:
        return os.path.abspath(configured)
    return os.path.normpath(
        os.path.join(core_crate_dir(), "..", "..", "bindings", "python", "tests", "fixtures")
    )
