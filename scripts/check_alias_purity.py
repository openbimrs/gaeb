#!/usr/bin/env python3
"""Fail closed unless `gaeb` is a semantic pure alias of `openbim-gaeb`."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def fail(message: str) -> "NoReturn":
    print(f"alias purity: {message}", file=sys.stderr)
    raise SystemExit(1)


def package(packages: list[dict], name: str) -> dict:
    matches = [candidate for candidate in packages if candidate["name"] == name]
    if len(matches) != 1:
        fail(f"expected exactly one {name!r} package, found {len(matches)}")
    return matches[0]


def normalized(path: str | Path) -> Path:
    return Path(path).resolve()


metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
packages = metadata["packages"]
canonical = package(packages, "openbim-gaeb")
alias = package(packages, "gaeb")

canonical_version = canonical["version"]
alias_version = alias["version"]
if alias_version != canonical_version:
    fail(
        f"package versions differ: gaeb={alias_version}, "
        f"openbim-gaeb={canonical_version}"
    )

expected_alias_manifest = normalized(ROOT / "gaeb/Cargo.toml")
if normalized(alias["manifest_path"]) != expected_alias_manifest:
    fail(f"gaeb manifest moved outside {expected_alias_manifest}")

if alias.get("features"):
    fail("gaeb must not define features")
if alias.get("links") is not None:
    fail("gaeb must not define a native links contract")

if len(alias["targets"]) != 1:
    fail("gaeb must contain exactly one Cargo target")
target = alias["targets"][0]
if target["kind"] != ["lib"] or target["crate_types"] != ["lib"]:
    fail("gaeb's only target must be a normal library")
if target["name"] != "gaeb":
    fail(f"gaeb library target has unexpected name {target['name']!r}")

source_path = normalized(target["src_path"])
expected_source_path = normalized(ROOT / "gaeb/src/lib.rs")
if source_path != expected_source_path:
    fail(f"gaeb library target must be {expected_source_path}, got {source_path}")

meaningful_lines = [
    line.strip()
    for line in source_path.read_text(encoding="utf-8").splitlines()
    if line.strip() and not line.lstrip().startswith("//")
]
if meaningful_lines != ["pub use openbim_gaeb::*;"]:
    fail("gaeb library must contain only `pub use openbim_gaeb::*;`")

dependencies = alias["dependencies"]
if len(dependencies) != 1:
    fail("gaeb must depend only on openbim-gaeb")
dependency = dependencies[0]
if dependency["name"] != "openbim-gaeb" or dependency.get("rename") is not None:
    fail("gaeb's sole dependency must be the unrenamed openbim-gaeb package")
if dependency.get("kind") is not None or dependency.get("optional"):
    fail("openbim-gaeb must be a required normal dependency")
expected_requirement = f"={canonical_version}"
if dependency["req"] != expected_requirement:
    fail(
        f"openbim-gaeb requirement must be {expected_requirement}, "
        f"got {dependency['req']}"
    )
expected_dependency_path = normalized(ROOT / "openbim-gaeb")
if dependency.get("path") is None:
    fail("openbim-gaeb must be a local path dependency for workspace validation")
if normalized(dependency["path"]) != expected_dependency_path:
    fail(
        f"openbim-gaeb path must resolve to {expected_dependency_path}, "
        f"got {dependency['path']}"
    )

