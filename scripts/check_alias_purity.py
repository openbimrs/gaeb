#!/usr/bin/env python3
"""Fail closed unless gaeb is a pure Cargo alias of openbim-gaeb."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import NoReturn

ROOT = Path(__file__).resolve().parent.parent
ALIAS_PACKAGE = "gaeb"
CANONICAL_PACKAGE = "openbim-gaeb"
CANONICAL_CRATE = "openbim_gaeb"



def fail(message: str) -> NoReturn:
    print(f"alias purity error: {message}", file=sys.stderr)
    raise SystemExit(1)


def normalize(path: str | Path) -> Path:
    return Path(path).resolve()


def package(packages: list[dict], name: str) -> dict:
    matches = [candidate for candidate in packages if candidate["name"] == name]
    if len(matches) != 1:
        fail(f"expected exactly one {name!r} package, found {len(matches)}")
    return matches[0]


def without_comments(source: str) -> str:
    output: list[str] = []
    index = 0
    block_depth = 0
    while index < len(source):
        pair = source[index:index + 2]
        if block_depth:
            if pair == "/*":
                block_depth += 1
                index += 2
            elif pair == "*/":
                block_depth -= 1
                index += 2
            else:
                index += 1
            continue
        if pair == "//":
            newline = source.find("\n", index + 2)
            if newline == -1:
                break
            output.append("\n")
            index = newline + 1
        elif pair == "/*":
            block_depth = 1
            index += 2
        elif pair == "*/":
            fail("unmatched block-comment terminator")
        else:
            output.append(source[index])
            index += 1
    if block_depth:
        fail("unterminated block comment")
    return "".join(output)


def rust_tokens(source: str) -> list[str]:
    pattern = re.compile(r"\s+|::|[A-Za-z_][A-Za-z0-9_]*|[*!;]")
    tokens: list[str] = []
    position = 0
    while position < len(source):
        match = pattern.match(source, position)
        if match is None:
            fail(f"unexpected alias source near {source[position:position + 20]!r}")
        token = match.group(0)
        if not token.isspace():
            tokens.append(token)
        position = match.end()
    return tokens


metadata = json.loads(subprocess.run(
    ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
    cwd=ROOT,
    check=True,
    capture_output=True,
    text=True,
).stdout)
packages = metadata["packages"]
package_names = {candidate["name"] for candidate in packages}
expected_names = {ALIAS_PACKAGE, CANONICAL_PACKAGE, "openbim-gaeb-bindings"}
if package_names != expected_names:
    fail(f"workspace package set must be {sorted(expected_names)}, got {sorted(package_names)}")

canonical = package(packages, CANONICAL_PACKAGE)
alias = package(packages, ALIAS_PACKAGE)
version = canonical["version"]
if alias["version"] != version:
    fail(f"alias version must match canonical version {version}")
if alias.get("edition") != "2021":
    fail("alias package edition must be 2021")
if alias.get("rust_version") != "1.85":
    fail("alias package rust-version must be 1.85")
if alias.get("publish") != []:
    fail("alias package publishing must remain disabled while canonical dependencies are Git-pinned")
if alias.get("features"):
    fail("alias must not define Cargo features")
if alias.get("links") is not None:
    fail("alias must not define a native links contract")
if alias.get("source") is not None:
    fail("workspace alias must be a local package")

expected_manifest = normalize(ROOT / ALIAS_PACKAGE / "Cargo.toml")
if normalize(alias["manifest_path"]) != expected_manifest:
    fail(f"alias manifest must be {expected_manifest}")

targets = alias["targets"]
if len(targets) != 1:
    fail(f"alias must contain exactly one target, found {len(targets)}")
target = targets[0]
if target["kind"] != ["lib"] or target["crate_types"] != ["lib"]:
    fail("alias target must be one ordinary library")
if target.get("edition") != "2021":
    fail("alias target edition must be 2021")
for flag in ("doc", "doctest", "test"):
    if target.get(flag) is not True:
        fail(f"alias target must keep {flag}=true")
if target.get("required-features", []) != []:
    fail("alias target must not require features")
if target["name"] != ALIAS_PACKAGE.replace("-", "_"):
    fail(f"unexpected alias library target name {target['name']!r}")
expected_source = normalize(ROOT / ALIAS_PACKAGE / "src/lib.rs")
source_path = normalize(target["src_path"])
if source_path != expected_source:
    fail(f"alias library source must be {expected_source}, got {source_path}")

expected_tokens = ["pub", "use", CANONICAL_CRATE, "::", "*", ";"]
tokens = rust_tokens(without_comments(source_path.read_text(encoding="utf-8")))
if tokens != expected_tokens:
    fail(f"alias source tokens must be exactly {expected_tokens!r}, got {tokens!r}")

extra_sources = sorted(
    path.relative_to(ROOT)
    for path in (ROOT / ALIAS_PACKAGE).rglob("*.rs")
    if normalize(path) != source_path
)
if extra_sources:
    fail(f"alias contains unexpected Rust sources: {extra_sources}")

dependencies = alias["dependencies"]
if len(dependencies) != 1:
    fail(f"alias must have exactly one dependency, found {len(dependencies)}")
dependency = dependencies[0]
if dependency["name"] != CANONICAL_PACKAGE or dependency.get("rename") is not None:
    fail(f"sole dependency must be unrenamed {CANONICAL_PACKAGE}")
expected_fields = {
    "kind": None,
    "optional": False,
    "target": None,
    "registry": None,
    "source": None,
    "uses_default_features": True,
    "features": [],
}
for field, expected in expected_fields.items():
    actual = dependency.get(field)
    if actual != expected:
        fail(f"canonical dependency field {field!r} must be {expected!r}, got {actual!r}")
expected_requirement = f"={version}"
if dependency["req"] != expected_requirement:
    fail(f"canonical requirement must be {expected_requirement}, got {dependency['req']}")
expected_path = normalize(ROOT / CANONICAL_PACKAGE)
if dependency.get("path") is None or normalize(dependency["path"]) != expected_path:
    fail(f"canonical dependency path must resolve to {expected_path}")

print(f"alias purity: {ALIAS_PACKAGE} is one unconditional exact-version pure re-export")
