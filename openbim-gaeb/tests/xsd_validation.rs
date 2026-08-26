use openbim_gaeb::{ValidationLayer, XsdSchema};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xsd")
        .join(name)
}

#[test]
fn xsd_accepts_a_structurally_valid_document() {
    let schema = XsdSchema::from_file(fixture("minimal.xsd")).unwrap();
    let report = schema
        .validate(
            br#"<?before ok?><Root xmlns="urn:openbim:gaeb:test"><!--kept--><?inside ok?><Value>42</Value></Root><?after ok?>"#,
        )
        .unwrap();

    assert!(report.is_valid(), "{:#?}", report.diagnostics());
    assert!(report.diagnostics().is_empty());
}

#[test]
fn xsd_rejects_invalid_content_with_distinguishable_diagnostics() {
    let schema = XsdSchema::from_file(fixture("minimal.xsd")).unwrap();
    let report = schema
        .validate(br#"<Root xmlns="urn:openbim:gaeb:test"><Value>not-an-int</Value></Root>"#)
        .unwrap();

    assert!(!report.is_valid());
    let diagnostic = &report.diagnostics()[0];
    assert_eq!(diagnostic.layer(), ValidationLayer::Xsd);
    assert_eq!(diagnostic.code(), "cvc-datatype-valid");
    assert_eq!(diagnostic.severity().as_str(), "error");
    assert!(
        diagnostic.line().is_some() || diagnostic.location().is_some(),
        "{diagnostic:#?}"
    );
}

#[test]
fn malformed_instance_is_an_xsd_layer_error_not_a_panic() {
    let schema = XsdSchema::from_file(fixture("minimal.xsd")).unwrap();
    let report = schema
        .validate(br#"<Root xmlns="urn:openbim:gaeb:test">"#)
        .unwrap();

    assert!(!report.is_valid());
    assert_eq!(report.diagnostics()[0].layer(), ValidationLayer::Xsd);
    assert_eq!(report.diagnostics()[0].code(), "XSD-XML-PARSE");
}

#[test]
fn xsd_rejects_every_document_level_well_formedness_violation() {
    let schema = XsdSchema::from_file(fixture("minimal.xsd")).unwrap();
    let malformed: &[&[u8]] = &[
        b"",
        b" \n\t",
        br#"<Root xmlns="urn:openbim:gaeb:test"/><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"outside<Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test"/>outside"#,
        br#"<!DOCTYPE Root><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml version="1.0"?><Root xmlns="urn:openbim:gaeb:test"/><?xml version="1.0"?>"#,
    ];
    for xml in malformed {
        let report = schema.validate(xml).unwrap();
        assert!(!report.is_valid(), "accepted malformed XML: {xml:?}");
        assert!(
            report.has_code("XSD-XML-PARSE"),
            "{:#?}",
            report.diagnostics()
        );
    }
}

#[test]
fn schema_loading_fails_closed_when_an_include_is_missing() {
    let directory = tempfile::tempdir().unwrap();
    let schema = directory.path().join("root.xsd");
    std::fs::write(
        &schema,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:include schemaLocation="missing.xsd"/><xs:element name="Root" type="xs:string"/></xs:schema>"#,
    )
    .unwrap();
    assert!(XsdSchema::from_file(schema).is_err());
}
