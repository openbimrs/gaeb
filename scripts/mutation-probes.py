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
    Probe(
        "namespace-isolation",
        "openbim-gaeb/src/parser.rs",
        "if !current.gaeb {",
        "if false && !current.gaeb {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "ignores_vendor_elements_that_reuse_gaeb_local_names"),
    ),
    Probe(
        "attribute-namespace-validation",
        "openbim-gaeb/src/parser.rs",
        "        resolved_namespace(reader.resolve_attribute(attribute.key).0)?;",
        "        let _ = reader.resolve_attribute(attribute.key);",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "validates_every_attribute_namespace_and_entity"),
    ),
    Probe(
        "xml-name-validation",
        "openbim-gaeb/src/parser.rs",
        "    validate_qname(start.name().as_ref(), \"element\")?;",
        "    let _ = start.name();",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_invalid_xml_element_names"),
    ),
    Probe(
        "exact-namespace-matrix",
        "openbim-gaeb/src/parser.rs",
        "        \"3.2\" if PHASES_3_2.contains(&phase) => GaebVersion::V3_2,",
        "        \"3.2\" if PHASES_3_3_AND_3_4.contains(&phase) => GaebVersion::V3_2,",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "rejects_nonexistent_namespace_version_phase_products"),
    ),
    Probe(
        "schema-item-scope",
        "openbim-gaeb/src/parser.rs",
        "                } else if gaeb && local == \"Item\" && direct_itemlist_item(&path) {\n                    if current_item.is_some() {",
        "                } else if gaeb && local == \"Item\" {\n                    if current_item.is_some() {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "extracts_only_schema_positioned_boq_items_and_categories"),
    ),
    Probe(
        "direct-description-scope",
        "openbim-gaeb/src/parser.rs",
        "        if in_direct_item_description(path) {",
        "        if path.iter().any(|element| element.is_gaeb(\"Description\")) {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "item_description_excludes_nested_subdescription_text"),
    ),
    Probe(
        "nested-quantity-semantics",
        "openbim-gaeb/src/parser.rs",
        "    fn invalidate_quantity_value(&mut self) {\n        self.quantity_ambiguous = true;\n        self.quantity = None;\n        self.quantity_fragments.clear();\n        self.quantity_has_non_value_xml = true;\n    }",
        "    fn invalidate_quantity_value(&mut self) {\n        self.block_quantity_edit();\n    }",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "nested_quantity_markup_is_not_exposed_as_a_fabricated_value"),
    ),
    Probe(
        "empty-declaration-tracking",
        "openbim-gaeb/src/parser.rs",
        "                track_metadata_declaration(&path, &mut declarations, &mut diagnostics);\n                if gaeb && local == \"Item\" && direct_itemlist_item(&path) {",
        "                if gaeb && local == \"Item\" && direct_itemlist_item(&path) {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "empty_version_and_phase_elements_still_count_as_declarations"),
    ),
    Probe(
        "duplicate-version-stability",
        "openbim-gaeb/src/parser.rs",
        "        \"Version\" if in_header && declarations.version == 1 => {",
        "        \"Version\" if in_header => {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "duplicate_version_and_phase_declarations_are_explicitly_diagnosed"),
    ),
    Probe(
        "phase-parent-scope",
        "openbim-gaeb/src/parser.rs",
        "            \"Award\"\n",
        "            \"Award\" | \"Anything\"\n",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "only_schema_defined_top_level_phase_locations_are_interpreted"),
    ),
    Probe(
        "fragmented-quantity-fail-closed",
        "openbim-gaeb/src/parser.rs",
        "} else if !self.quantity_has_non_value_xml && self.quantity_fragments.len() == 1 {",
        "} else if !self.quantity_fragments.is_empty() {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "quantity_comments_are_read_completely_but_edits_fail_closed"),
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
