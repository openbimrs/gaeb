#!/usr/bin/env python3
"""Package all three crates and compile dependents against the canonical candidate."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
XSD_SCHEMA_GIT = "https://github.com/GeneralPawz/xsd-schema.git"
XSD_SCHEMA_REV = "53de66ccb075246a67e5986742cdcdb5deb81267"
XSD_PATCH_ARGS = (
    "--config",
    f'patch.crates-io.xsd-schema.git="{XSD_SCHEMA_GIT}"',
    "--config",
    f'patch.crates-io.xsd-schema.rev="{XSD_SCHEMA_REV}"',
)
if sys.argv[1:] not in ([], ["--allow-dirty"]):
    raise SystemExit("usage: verify-packages.py [--allow-dirty]")
PACKAGE_DIRTY_ARGS = ("--allow-dirty",) if sys.argv[1:] else ()


def run(*command: str, cwd: Path = ROOT, capture: bool = False) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if result.stdout:
            print(result.stdout, file=sys.stderr)
        raise SystemExit(result.returncode)
    return result


metadata = json.loads(
    run(
        "cargo",
        "metadata",
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
        capture=True,
    ).stdout
)
versions = {package["name"]: package["version"] for package in metadata["packages"]}
for package in metadata["packages"]:
    if (
        package["name"] in {"openbim-gaeb", "openbim-gaeb-bindings", "gaeb"}
        and package["publish"] != []
    ):
        raise SystemExit(
            f'{package["name"]} publishing must stay disabled while xsd-schema is Git-pinned'
        )
canonical_version = versions["openbim-gaeb"]
bindings_version = versions["openbim-gaeb-bindings"]
alias_version = versions["gaeb"]
canonical_manifest = tomllib.loads((ROOT / "openbim-gaeb" / "Cargo.toml").read_text())
manifest_xsd_rev = canonical_manifest["dependencies"]["xsd-schema"].get("rev")
if manifest_xsd_rev != XSD_SCHEMA_REV:
    raise SystemExit(
        f"package verifier xsd-schema revision {XSD_SCHEMA_REV} does not match manifest {manifest_xsd_rev}"
    )
if canonical_version != alias_version:
    raise SystemExit("canonical and alias package versions differ")

run(
    "cargo",
    "package",
    "--locked",
    *PACKAGE_DIRTY_ARGS,
    "-p",
    "openbim-gaeb",
    *XSD_PATCH_ARGS,
)
run(
    "cargo",
    "package",
    "--locked",
    *PACKAGE_DIRTY_ARGS,
    "--no-verify",
    "-p",
    "openbim-gaeb-bindings",
    "--config",
    f'patch.crates-io.openbim-gaeb.path="{(ROOT / "openbim-gaeb").as_posix()}"',
)
run(
    "cargo",
    "package",
    "--locked",
    *PACKAGE_DIRTY_ARGS,
    "--no-verify",
    "-p",
    "gaeb",
    "--config",
    f'patch.crates-io.openbim-gaeb.path="{(ROOT / "openbim-gaeb").as_posix()}"',
)

package_root = TARGET / "package"
canonical_source = package_root / f"openbim-gaeb-{canonical_version}"
bindings_source = package_root / f"openbim-gaeb-bindings-{bindings_version}"
alias_source = package_root / f"gaeb-{alias_version}"
for source in (bindings_source, alias_source):
    if source.exists():
        shutil.rmtree(source)
for crate_name, version, source in (
    ("openbim-gaeb", canonical_version, canonical_source),
    ("openbim-gaeb-bindings", bindings_version, bindings_source),
    ("gaeb", alias_version, alias_source),
):
    if not source.is_dir():
        archive = package_root / f"{crate_name}-{version}.crate"
        if not archive.is_file():
            raise SystemExit(f"Cargo package archive missing: {archive}")
        with tarfile.open(archive, "r:gz") as package:
            package.extractall(package_root, filter="data")
    if not source.is_dir():
        raise SystemExit(f"Cargo package extraction missing: {source}")

for candidate_source in (canonical_source, bindings_source, alias_source):
    config_dir = candidate_source / ".cargo"
    config_dir.mkdir(exist_ok=True)
    patch_lines = [
        "[patch.crates-io]",
        f'xsd-schema = {{ git = "{XSD_SCHEMA_GIT}", rev = "{XSD_SCHEMA_REV}" }}',
    ]
    if candidate_source != canonical_source:
        patch_lines.append(
            f'openbim-gaeb = {{ path = "{canonical_source.as_posix()}" }}'
        )
    (config_dir / "config.toml").write_text(
        "\n".join(patch_lines) + "\n",
        encoding="utf-8",
    )
    run(
        "cargo",
        "test",
        "--manifest-path",
        str(candidate_source / "Cargo.toml"),
        cwd=candidate_source,
    )

for package_name, candidate_source in (
    ("openbim-gaeb-bindings", bindings_source),
    ("gaeb", alias_source),
):
    resolved = json.loads(
        run(
            "cargo",
            "metadata",
            "--manifest-path",
            str(candidate_source / "Cargo.toml"),
            "--format-version",
            "1",
            cwd=candidate_source,
            capture=True,
        ).stdout
    )
    resolved_canonical = next(
        package for package in resolved["packages"] if package["name"] == "openbim-gaeb"
    )
    if Path(resolved_canonical["manifest_path"]).resolve() != (
        canonical_source / "Cargo.toml"
    ).resolve():
        raise SystemExit(
            f"packaged {package_name} did not resolve the candidate canonical package"
        )

print(
    f"package verification passed: gaeb {alias_version} and "
    f"openbim-gaeb-bindings {bindings_version} compiled against "
    f"candidate openbim-gaeb {canonical_version}"
)
