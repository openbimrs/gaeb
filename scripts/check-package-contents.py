#!/usr/bin/env python3
"""Require exact Cargo package payloads; fail on additions and omissions."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = {
    "openbim-gaeb": [
        ".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig",
        "LICENSE", "LICENSES/MIT.txt", "README.md", "build.rs", "examples/inspect.rs",
        "schema-support-matrix.json", "src/business/decimal.rs",
        "src/business/mod.rs", "src/business/pair.rs", "src/business/single.rs",
        "src/business/tree.rs", "src/diagnostic.rs", "src/document.rs",
        "src/error.rs", "src/lib.rs", "src/metadata.rs", "src/model.rs",
        "src/parser.rs", "src/phase.rs", "src/support.rs", "src/validation.rs",
        "src/version.rs", "src/xsd/collection.rs", "src/xsd/single.rs", "src/xsd.rs",
        "support-matrix.csv", "tests/business_rule_pairs.rs",
        "tests/business_validation.rs", "tests/detection.rs", "tests/document.rs",
        "tests/editing.rs", "tests/fixtures/xsd/minimal.xsd",
        "tests/official_corpus.rs", "tests/official_xsd.rs",
        "tests/support_matrix.rs", "tests/xsd_validation.rs",
    ],
    "openbim-gaeb-bindings": [
        ".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig",
        "LICENSE", "LICENSES/MIT.txt", "README.md", "src/generated/v3_1_2007_11.rs", "src/lib.rs",
    ],
    "gaeb": [
        ".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig",
        "LICENSE", "LICENSES/MIT.txt", "README.md", "src/lib.rs",
    ],
}

errors: list[str] = []
for package, expected in EXPECTED.items():
    result = subprocess.run(
        ["cargo", "package", "--locked", "--allow-dirty", "--list", "-p", package],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    actual = sorted(line for line in result.stdout.splitlines() if line)
    wanted = sorted(expected)
    missing = sorted(set(wanted) - set(actual))
    unexpected = sorted(set(actual) - set(wanted))
    if missing or unexpected or actual != wanted:
        errors.append(
            f"{package} payload mismatch: missing={missing}, unexpected={unexpected}"
        )
if errors:
    for error in errors:
        print(f"package contents error: {error}", file=sys.stderr)
    raise SystemExit(1)
print("package contents: exact allowlists verified")
