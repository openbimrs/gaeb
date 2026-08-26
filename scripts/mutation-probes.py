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
        "        ResolveResult::Unknown(prefix) => Err(Error::Xml(format!(\n            \"undeclared XML namespace prefix {:?}\",\n            String::from_utf8_lossy(&prefix)\n        ))),",
        "        ResolveResult::Unknown(_prefix) => Ok(None),",
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
        "fn direct_itemlist_item(path: &[PathElement], namespace: &str) -> bool {\n    path.len() >= 6\n        && path[path.len() - 2].is_gaeb(\"Itemlist\")\n        && path[path.len() - 1].is_gaeb(\"Item\")\n        && valid_boq_descendant_path(&path[..path.len() - 2], namespace)\n}",
        "fn direct_itemlist_item(path: &[PathElement], _namespace: &str) -> bool {\n    path.len() >= 2\n        && path[path.len() - 2].is_gaeb(\"Itemlist\")\n        && path[path.len() - 1].is_gaeb(\"Item\")\n}",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "extracts_only_schema_positioned_boq_items_and_categories"),
    ),
    Probe(
        "direct-description-scope",
        "openbim-gaeb/src/parser.rs",
        "        if in_direct_item_description(path, item.item_depth) {",
        "        if path.iter().any(|element| element.is_gaeb(\"Description\")) {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "item_description_excludes_nested_subdescription_text"),
    ),
    Probe(
        "nested-quantity-semantics",
        "openbim-gaeb/src/parser.rs",
        "    fn invalidate_quantity_value(&mut self) {\n        self.quantity_seen = true;\n        self.quantity_ambiguous = true;\n        self.quantity = None;\n        self.quantity_fragments.clear();\n        self.quantity_has_non_value_xml = true;\n    }",
        "    fn invalidate_quantity_value(&mut self) {\n        self.quantity_seen = true;\n        self.block_quantity_edit();\n    }",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "nested_quantity_markup_is_not_exposed_as_a_fabricated_value"),
    ),
    Probe(
        "empty-declaration-tracking",
        "openbim-gaeb/src/parser.rs",
        "    if current.is_gaeb(\"Version\") && direct_header_child(path) {",
        "    if false && current.is_gaeb(\"Version\") && direct_header_child(path) {",
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
        "        \"31\" => \"QtyDeterm\",",
        "        \"31\" => \"Award\",",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "phase_declarations_require_the_product_specific_gaeb_parent"),
    ),
    Probe(
        "expanded-attribute-uniqueness",
        "openbim-gaeb/src/parser.rs",
        "        if !is_namespace_declaration\n            && !expanded_names.insert((namespace, attribute.key.local_name().as_ref().to_vec()))",
        "        if false\n            && !is_namespace_declaration\n            && !expanded_names.insert((namespace, attribute.key.local_name().as_ref().to_vec()))",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_duplicate_expanded_attribute_names"),
    ),
    Probe(
        "namespace-binding-constraints",
        "openbim-gaeb/src/parser.rs",
        "            validate_namespace_declaration(name, decoded.as_ref())?;",
        "            let _ = (name, decoded.as_ref());",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_namespace_constraint_violations"),
    ),
    Probe(
        "text-line-ending-normalization",
        "openbim-gaeb/src/parser.rs",
        "            normalized.push('\\n');",
        "            normalized.push('\\r');",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "normalizes_xml_semantics_without_changing_source_bytes"),
    ),
    Probe(
        "attribute-value-normalization",
        "openbim-gaeb/src/parser.rs",
        "            '\\n' | '\\t' => normalized.push(' '),",
        "            '\\n' | '\\t' => normalized.push(character),",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "normalizes_xml_semantics_without_changing_source_bytes"),
    ),
    Probe(
        "pi-target-namespace-grammar",
        "openbim-gaeb/src/parser.rs",
        "    validate_xml_name(target, false, \"processing instruction target\")?;",
        "    validate_xml_name(target, true, \"processing instruction target\")?;",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_other_xml_lexical_malformations"),
    ),
    Probe(
        "namespace-reference-normalization",
        "openbim-gaeb/src/parser.rs",
        "            let decoded = unescape(normalized.as_ref())\n                .map_err(|error| Error::Xml(format!(\"invalid namespace entity: {error}\")))?;",
        "            let decoded: Cow<'_, str> = Cow::Borrowed(normalized.as_ref());",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "detection", "resolves_character_references_in_namespace_declarations"),
    ),
    Probe(
        "active-item-depth-scope",
        "openbim-gaeb/src/parser.rs",
        "fn direct_item_child(path: &[PathElement], item_depth: usize) -> bool {\n    path.len() == item_depth + 1 && path[item_depth - 1].is_gaeb(\"Item\")\n}",
        "fn direct_item_child(path: &[PathElement], _item_depth: usize) -> bool {\n    path.len() >= 2 && path[path.len() - 2].is_gaeb(\"Item\")\n}",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "nested_item_fields_do_not_attach_to_the_active_schema_item"),
    ),
    Probe(
        "empty-quantity-state",
        "openbim-gaeb/src/parser.rs",
        "                        if direct_item_child(&path, item.item_depth) {\n                            item.invalidate_quantity_value();\n                        }",
        "                        if false && direct_item_child(&path, item.item_depth) {\n                            item.invalidate_quantity_value();\n                        }",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "empty_quantity_is_existing_but_not_editable"),
    ),
    Probe(
        "fragmented-quantity-fail-closed",
        "openbim-gaeb/src/parser.rs",
        "} else if !self.quantity_has_non_value_xml && self.quantity_fragments.len() == 1 {",
        "} else if !self.quantity_fragments.is_empty() {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "quantity_comments_are_read_completely_but_edits_fail_closed"),
    ),
    Probe(
        "explicit-empty-quantity-state",
        "openbim-gaeb/src/parser.rs",
        "        } else if self.quantity_ambiguous || quantity.is_none() {\n            QuantityEdit::NotEditable",
        "        } else if self.quantity_ambiguous {\n            QuantityEdit::NotEditable\n        } else if quantity.is_none() {\n            QuantityEdit::Missing",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "editing", "empty_quantity_is_existing_but_not_editable"),
    ),
    Probe(
        "xml-space-outside-root",
        "openbim-gaeb/src/parser.rs",
        "                    if raw.bytes().all(is_xml_space) {",
        "                    if raw.trim().is_empty() {",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_non_xml_whitespace_outside_the_root"),
    ),
    Probe(
        "namespace-uri-reference",
        "openbim-gaeb/src/parser.rs",
        "    if !value.is_empty() && UriReferenceStr::new(value).is_err() {\n        return Err(Error::Xml(\n            \"an XML namespace name must be a valid URI reference\".into(),\n        ));\n    }",
        "    let _ = value;",
        ("cargo", "test", "-p", "openbim-gaeb", "--test", "document", "rejects_namespace_names_that_are_not_uri_references"),
    ),
    Probe(
        'xsd-document-shape',
        'openbim-gaeb/src/xsd/single.rs',
        '        if let Err(message) = validate_xml_document_shape(xml) {',
        '        if let Err(message) = validate_xml_document_shape(b"<Root/>") {',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-comment-lexical-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'reader.config_mut().check_comments = true;',
        'reader.config_mut().check_comments = false;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-declaration-lexical-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'validate_xml_declaration(&declaration)?;',
        'let _ = &declaration;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-pi-target-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'if !is_valid_pi_target(&target) {',
        'if false && !is_valid_pi_target(&target) {',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-qname-lexical-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'is_valid_ncname(first) && second.is_none_or(is_valid_ncname)',
        'true',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-leading-whitespace-before-declaration',
        'openbim-gaeb/src/xsd/single.rs',
        '                    if !saw_root {\n                        saw_prolog_content = true;\n                    }',
        '                    if !saw_root {\n                        saw_prolog_content = false;\n                    }',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-xml10-content-character-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'unescaped.chars().all(is_xml10_char)',
        'true',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-xml10-raw-character-validation',
        'openbim-gaeb/src/xsd/single.rs',
        'decoded.chars().all(is_xml10_char)',
        'true',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-attribute-fanout-budget',
        'openbim-gaeb/src/xsd/single.rs',
        'const MAX_ATTRIBUTES_PER_ELEMENT: usize = 1_024;',
        'const MAX_ATTRIBUTES_PER_ELEMENT: usize = usize::MAX;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_bounds_pathological_attribute_and_diagnostic_fanout'),
    ),
    Probe(
        'xsd-diagnostic-fanout-budget',
        'openbim-gaeb/src/xsd/single.rs',
        'const MAX_XSD_DIAGNOSTICS: usize = 4_096;',
        'const MAX_XSD_DIAGNOSTICS: usize = usize::MAX;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_bounds_pathological_attribute_and_diagnostic_fanout'),
    ),
    Probe(
        'xsd-unresolved-namespace-import',
        'openbim-gaeb/src/xsd/single.rs',
        '.imports\n                    .iter()\n                    .any(|directive| directive.resolved_doc_id.is_none())',
        '.imports\n                    .iter()\n                    .any(|_| false)',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'schema_loading_fails_closed_for_namespace_only_imports'),
    ),
    Probe(
        'xsd-namespace-binding-constraints',
        'openbim-gaeb/src/xsd/single.rs',
        '    if prefix == "xmlns" {',
        '    if false && prefix == "xmlns" {',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'xsd_rejects_every_document_level_well_formedness_violation'),
    ),
    Probe(
        'xsd-schema-root-confinement',
        'openbim-gaeb/src/xsd/single.rs',
        '                Component::ParentDir | Component::RootDir | Component::Prefix(_)',
        '                Component::RootDir | Component::Prefix(_)',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'schema_loading_confines_directives_to_the_root_directory'),
    ),
    Probe(
        'xsd-schema-graph-depth-budget',
        'openbim-gaeb/src/xsd/single.rs',
        'const MAX_SCHEMA_GRAPH_DEPTH: usize = 64;',
        'const MAX_SCHEMA_GRAPH_DEPTH: usize = usize::MAX;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'schema_loading_bounds_graph_depth'),
    ),
    Probe(
        'xsd-schema-graph-document-budget',
        'openbim-gaeb/src/xsd/single.rs',
        'const MAX_SCHEMA_DOCUMENTS: usize = 256;',
        'const MAX_SCHEMA_DOCUMENTS: usize = usize::MAX;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'schema_loading_bounds_graph_cardinality'),
    ),
    Probe(
        'xsd-schema-graph-byte-budget',
        'openbim-gaeb/src/xsd/single.rs',
        'const MAX_SCHEMA_GRAPH_BYTES: usize = 8 * 1024 * 1024;',
        'const MAX_SCHEMA_GRAPH_BYTES: usize = usize::MAX;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'xsd_validation', 'schema_loading_bounds_total_graph_bytes'),
    ),
    Probe(
        'typed-exact-decimal',
        'openbim-gaeb-bindings/src/generated/v3_1_2007_11.rs',
        'pub type TgDecimalType = ExactDecimal;\npub type TgDecimal113Type = ExactDecimal;\npub type TgDecimal132Type = ExactDecimal;\npub type TgDecimal133Type = ExactDecimal;\npub type TgDecimal52Type = ExactDecimal;\npub type TgDecimal64Type = ExactDecimal;',
        'pub type TgDecimalType = f64;\npub type TgDecimal113Type = f64;\npub type TgDecimal132Type = f64;\npub type TgDecimal133Type = f64;\npub type TgDecimal52Type = f64;\npub type TgDecimal64Type = f64;',
        ('cargo', 'test', '-p', 'openbim-gaeb-bindings', '--test', 'exact_decimal', 'all_gaeb_decimal_aliases_preserve_values_beyond_binary64_integer_precision'),
    ),
    Probe(
        'typed-numeric-attribute-retention',
        'openbim-gaeb-bindings/tests/official_roundtrip.rs',
        'values.push((format!("{}/@{name}", path.join("/")), decimal));',
        'drop((name, decimal));',
        ('cargo', 'test', '-p', 'openbim-gaeb-bindings', '--test', 'official_roundtrip', 'numeric_leaf_values_include_attributes'),
    ),
    Probe(
        'typed-xml-space-namespace',
        'openbim-gaeb-bindings/src/generated/v3_1_2007_11.rs',
        'helper.write_attrib(&mut bytes, "xml:space", &self.value.space)?;',
        'helper.write_attrib(&mut bytes, "space", &self.value.space)?;',
        ('cargo', 'test', '-p', 'openbim-gaeb-bindings', '--test', 'exact_decimal', 'generated_serializer_qualifies_xml_space'),
    ),
    Probe(
        'business-warning-severity',
        'openbim-gaeb/src/business/mod.rs',
        '        severity: ValidationSeverity::Warning,',
        '        severity: ValidationSeverity::Error,',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'catalog_distinguishes_evidence_backed_rules_from_interoperability_lints'),
    ),
    Probe(
        'pair-release-coherence',
        'openbim-gaeb/src/business/pair.rs',
        '    if !same_release(baseline, candidate) {',
        '    if false && !same_release(baseline, candidate) {',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'pair_lints_require_a_coherent_release_tuple'),
    ),
    Probe(
        'discount-arithmetic',
        'openbim-gaeb/src/business/single.rs',
        '                        .and_then(|discount| qty.multiply_discounted_rounded(&up, &discount, 2)),',
        '                        .and_then(|_| qty.multiply_rounded(&up, 2)),',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'item_total_accounts_for_item_discount_percentage'),
    ),
    Probe(
        'boq-component-scope',
        'openbim-gaeb/src/business/single.rs',
        '        if tree.is(id, "BoQ") {',
        '        if false && tree.is(id, "BoQ") {',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'unit_price_component_counts_are_scoped_to_each_boq'),
    ),
    Probe(
        'component-count-overflow',
        'openbim-gaeb/src/business/single.rs',
        '    Some(value.parse().unwrap_or(usize::MAX))',
        '    value.parse().ok()',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'component_count_is_bounded_and_owned_only_by_the_nearest_boq'),
    ),
    Probe(
        'business-arbitrary-decimal',
        'openbim-gaeb/src/business/decimal.rs',
        '        let mut units = BigInt::parse_bytes(digits.as_bytes(), 10)?;',
        '        let mut units = BigInt::from(digits.parse::<i128>().ok()?);',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'price_arithmetic_does_not_skip_xsd_valid_values_beyond_i128'),
    ),
    Probe(
        'text-complement-marker',
        'openbim-gaeb/src/business/pair.rs',
        '            .attribute(complement, "MarkLbl")',
        '            .attribute(complement, "Designation")',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'x83_x84_vat_and_text_complement_rules_are_pairwise'),
    ),
    Probe(
        'text-complement-baseline-designation',
        'openbim-gaeb/src/business/pair.rs',
        '            .is_some_and(|marker| designated_complements.contains(marker))',
        '            .is_some()',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'x83_x84_vat_and_text_complement_rules_are_pairwise'),
    ),
    Probe(
        'description-id-signature',
        'openbim-gaeb/src/business/pair.rs',
        'tree.attribute(description, "ID").unwrap_or("")',
        '""',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'x83_x84_detects_description_id_and_breakdown_changes'),
    ),
    Probe(
        'breakdown-signature',
        'openbim-gaeb/src/business/pair.rs',
        '.all_named("BoQBkdn")',
        '.all_named("__BoQBkdn")',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'x83_x84_detects_description_id_and_breakdown_changes'),
    ),
    Probe(
        'error-x84-release-coherence',
        'openbim-gaeb/src/business/single.rs',
        '    if metadata.namespace != "http://www.gaeb.de/GAEB_DA_XML/200407"\n        || metadata.version_text.as_deref() != Some("3.1")',
        '    if false\n        || metadata.version_text.as_deref() != Some("3.1")',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'conformance_errors_require_coherent_evidence_backed_releases'),
    ),
    Probe(
        'error-cost-phase-coherence',
        'openbim-gaeb/src/business/single.rs',
        '        && phase_from_namespace == phase_family',
        '        && true',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'conformance_errors_require_coherent_evidence_backed_releases'),
    ),
    Probe(
        'error-cost-date-coherence',
        'openbim-gaeb/src/business/single.rs',
        '        && metadata.version_date.as_deref() == Some("2021-05")',
        '        && true',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'conformance_errors_require_coherent_evidence_backed_releases'),
    ),
    Probe(
        'component-budget-exact-boundary',
        'openbim-gaeb/src/business/single.rs',
        'const MAX_UNIT_PRICE_COMPONENTS: usize = 6;',
        'const MAX_UNIT_PRICE_COMPONENTS: usize = 7;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_validation', 'component_limit_accepts_six_and_rejects_seven'),
    ),
    Probe(
        'pair-phase-namespace-coherence',
        'openbim-gaeb/src/business/pair.rs',
        'before.namespace_version == before.declared_version && phase_matches_namespace(baseline);',
        'before.namespace_version == before.declared_version;',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'business_rule_pairs', 'pair_lints_require_a_coherent_release_tuple'),
    ),
    Probe(
        'provenance-windows-forward-slash-path',
        'openbim-gaeb/schema-support-matrix.json',
        '"audited_repository": "https://github.com/openbimrs/gaeb"',
        '"audited_repository": "C:/Users/reviewer/schema"',
        ('cargo', 'test', '-p', 'openbim-gaeb', '--test', 'support_matrix', 'packaged_manifest_provenance_is_portable_and_not_self_stale'),
    ),
)


def run(
    *args: str,
    cwd: Path,
    capture: bool = False,
    target: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "CARGO_BUILD_JOBS": "2"}
    if target is not None:
        env["CARGO_TARGET_DIR"] = str(target)
    return subprocess.run(
        args,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
        env=env,
    )


def main() -> int:
    if run("git", "diff", "--quiet", "HEAD", "--", cwd=ROOT).returncode != 0:
        print("mutation probes require a clean tracked working tree")
        return 2

    temporary = Path(tempfile.mkdtemp(prefix="gaeb-mutation-", dir=CACHE if CACHE.is_dir() else None))
    survived: list[str] = []
    try:
        worktree = temporary / "worktree"
        target = temporary / "target"
        added = run("git", "worktree", "add", "--quiet", "--detach", str(worktree), "HEAD", cwd=ROOT)
        if added.returncode != 0:
            print("mutation worktree setup failed")
            return 2
        for probe in PROBES:
            try:
                path = worktree / probe.relative_path
                source = path.read_text()
                if source.count(probe.old) != 1:
                    print(f"{probe.name}: mutation anchor drifted")
                    return 2
                path.write_text(source.replace(probe.old, probe.new))
                compile_result = run(
                    "cargo",
                    "test",
                    "-p",
                    "openbim-gaeb",
                    "--tests",
                    "--no-run",
                    cwd=worktree,
                    capture=True,
                    target=target,
                )
                if compile_result.returncode != 0:
                    print(f"{probe.name}: mutated candidate did not compile")
                    print(compile_result.stdout)
                    return 2
                result = run(*probe.test, cwd=worktree, capture=True, target=target)
                if result.returncode == 0:
                    survived.append(probe.name)
                    print(f"{probe.name}: SURVIVED")
                elif "test result: FAILED" in (result.stdout or ""):
                    print(f"{probe.name}: killed by assertion failure")
                else:
                    print(f"{probe.name}: test command failed outside an assertion")
                    print(result.stdout)
                    return 2
            finally:
                path.write_text(source)
        if run("git", "diff", "--quiet", "HEAD", "--", cwd=worktree).returncode != 0:
            print("mutation worktree was not restored")
            return 2
    finally:
        if worktree.exists():
            run("git", "worktree", "remove", "--force", str(worktree), cwd=ROOT)
        run("git", "worktree", "prune", cwd=ROOT)
        shutil.rmtree(temporary, ignore_errors=True)

    if survived:
        print("surviving mutations: " + ", ".join(survived))
        return 1
    print(f"all {len(PROBES)} mutations killed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
