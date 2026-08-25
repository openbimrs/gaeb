#!/usr/bin/env python3
"""Prove that critical GAEB regression tests reject representative mutations."""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CACHE = Path("/mnt/backup/build-cache")
TARGET = CACHE / "openbim-gaeb-mutation-target" if CACHE.is_dir() else ROOT / "target" / "mutation"


@dataclass(frozen=True)
class Probe:
    name: str
    relative_path: str
    old: str
    new: str
    test: tuple[str, ...]


PROBES = (
    Probe(
        "version-conflict",
        "openbim-gaeb/src/parser.rs",
        "if namespace != declared {\n            diagnostics.push(Diagnostic::new(\n                DiagnosticKind::VersionMismatch,",
        "if namespace == declared {\n            diagnostics.push(Diagnostic::new(\n                DiagnosticKind::VersionMismatch,",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "surfaces_namespace_and_payload_disagreement"),
    ),
    Probe(
        "decimal-validation",
        "openbim-gaeb/src/document.rs",
        "    digits > 0\n}",
        "    true\n}",
        ("cargo", "test", "-p", "openbim-gaeb", "decimal_lexical_space_matches_xml_schema_shape"),
    ),
    Probe(
        "bom-preservation",
        "openbim-gaeb/src/document.rs",
        "            bytes: bytes.to_vec(),",
        "            bytes: xml.to_vec(),",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "unchanged_write_is_byte_identical_including_bom_and_unknown_xml"),
    ),
)


def run(*args: str, cwd: Path, capture: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
        env={**os.environ, "CARGO_TARGET_DIR": str(TARGET), "CARGO_BUILD_JOBS": "2"},
    )


def main() -> int:
    if run("git", "diff", "--quiet", "HEAD", "--", cwd=ROOT).returncode != 0:
        print("mutation probes require a clean tracked working tree")
        return 2

    temporary = Path(tempfile.mkdtemp(prefix="gaeb-mutation-", dir=CACHE if CACHE.is_dir() else None))
    survived: list[str] = []
    try:
        for index, probe in enumerate(PROBES):
            worktree = temporary / f"probe-{index}"
            added = run("git", "worktree", "add", "--quiet", "--detach", str(worktree), "HEAD", cwd=ROOT)
            if added.returncode != 0:
                print(f"{probe.name}: setup failed")
                return 2
            try:
                path = worktree / probe.relative_path
                source = path.read_text()
                if source.count(probe.old) != 1:
                    print(f"{probe.name}: mutation anchor drifted")
                    return 2
                path.write_text(source.replace(probe.old, probe.new))
                result = run(*probe.test, cwd=worktree, capture=True)
                if result.returncode == 0:
                    survived.append(probe.name)
                    print(f"{probe.name}: SURVIVED")
                else:
                    print(f"{probe.name}: killed")
            finally:
                run("git", "worktree", "remove", "--force", str(worktree), cwd=ROOT)
        run("git", "worktree", "prune", cwd=ROOT)
    finally:
        shutil.rmtree(temporary, ignore_errors=True)

    if survived:
        print("surviving mutations: " + ", ".join(survived))
        return 1
    print(f"all {len(PROBES)} mutations killed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
