use openbim_gaeb::{support, support::SUPPORT_MATRIX, Document};
use serde_json::Value;
use std::collections::HashSet;

fn manifest() -> Value {
    serde_json::from_str(include_str!("../schema-support-matrix.json"))
        .expect("schema support matrix must be valid JSON")
}

#[test]
fn comprehensive_manifest_has_reviewed_cardinality_and_unique_rows() {
    let root = manifest();
    let manifest = &root["manifest"];
    let counts = &manifest["counts"];
    assert_eq!(counts["semantic_support_rows"], 87);
    assert_eq!(counts["dispatch_edges"], 93);
    assert_eq!(counts["active_phase_roots"], 80);
    assert_eq!(counts["fixture_backed_rows"], 8);
    assert_eq!(counts["schema_only_rows"], 79);
    assert_eq!(
        manifest["claim_semantics"]["runtime_fixture_dispatch_key"],
        serde_json::json!(["generation", "namespace", "version_date", "phase"])
    );

    let rows = manifest["support_rows"].as_array().unwrap();
    let ids: HashSet<_> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(rows.len(), ids.len(), "duplicate semantic support row id");
    assert_eq!(
        rows.iter()
            .filter(|row| row["evidence"] == "official_fixture_and_schema")
            .count(),
        8
    );
}

#[test]
fn da84z_is_never_collapsed_into_da84() {
    let root = manifest();
    let rows = root["manifest"]["support_rows"].as_array().unwrap();
    for generation in ["3.2", "3.3", "3.4"] {
        let row = rows
            .iter()
            .find(|row| row["generation"] == generation && row["phase"] == "84Z")
            .unwrap();
        assert!(row["namespace"].as_str().unwrap().contains("DA84Z/"));
        assert!(!row["namespace"].as_str().unwrap().contains("/DA84/"));
    }
}

#[test]
fn runtime_matrix_claims_only_proven_capabilities() {
    assert_eq!(
        SUPPORT_MATRIX.len(),
        8,
        "fixture-backed XSD profile count drift"
    );
    let typed: Vec<_> = SUPPORT_MATRIX
        .iter()
        .filter(|entry| entry.typed_module.is_some())
        .collect();
    assert_eq!(
        typed.len(),
        3,
        "typed claims require parse/write/reparse proof"
    );
    assert!(typed
        .iter()
        .all(|entry| matches!(entry.phase.as_code(), "81" | "83" | "86")));
}

#[test]
fn dispatch_requires_exact_version_date() {
    fn document(version_date: &str) -> Document {
        let xml = format!(
            r#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/200407"><GAEBInfo><Version>3.1</Version><VersDate>{version_date}</VersDate></GAEBInfo><Award><DP>81</DP></Award></GAEB>"#,
        );
        Document::parse(xml.as_bytes()).unwrap()
    }

    assert_eq!(
        support::candidates_for_document(&document("2007-06")).count(),
        1
    );
    assert_eq!(
        support::candidates_for_document(&document("2099-01")).count(),
        0
    );
    assert_eq!(support::candidates_for_document(&document("")).count(), 0);
}
