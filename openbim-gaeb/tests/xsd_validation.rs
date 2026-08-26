use openbim_gaeb::{ValidationLayer, XsdSchema};
use std::{fmt::Write as _, path::PathBuf};

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
        b" \n<?xml version=\"1.0\"?><Root xmlns=\"urn:openbim:gaeb:test\"/>",
        br#"<!--bad--comment--><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml bad?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml version="2.0"?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml version="1.0" bogus="x"?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml version="1.0" standalone="maybe"?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?xml encoding="UTF-8" version="1.0"?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<?1bad data?><Root xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<1bad xmlns="urn:openbim:gaeb:test"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" 1bad="x"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:1bad="urn:x"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:xml="urn:not-xml"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:xmlns="urn:x"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:p="http://www.w3.org/XML/1998/namespace"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:p="http://www.w3.org/2000/xmlns/"/>"#,
        br#"<Root xmlns="urn:openbim:gaeb:test" xmlns:p=""/>"#,
        br#"<p:Root xmlns="urn:openbim:gaeb:test"/>"#,
        b"<Root xmlns=\"urn:openbim:gaeb:test\">\x01</Root>",
        br#"<Root xmlns="urn:openbim:gaeb:test"><Value>&#x1;</Value></Root>"#,
        b"<Root xmlns=\"urn:openbim:gaeb:test\" value=\"\x01\"/>",
        b"<!--\x01--><Root xmlns=\"urn:openbim:gaeb:test\"/>",
        b"<?ok \x01?><Root xmlns=\"urn:openbim:gaeb:test\"/>",
    ];
    for xml in malformed {
        let report = schema.validate(xml).unwrap();
        assert!(!report.is_valid(), "accepted malformed XML: {xml:?}");
        assert!(
            report.has_code("XSD-XML-PARSE"),
            "accepted malformed XML without parse diagnostic: {xml:?}: {:#?}",
            report.diagnostics()
        );
    }
}

#[test]
fn xsd_bounds_pathological_attribute_and_diagnostic_fanout() {
    let schema = XsdSchema::from_file(fixture("minimal.xsd")).unwrap();

    let mut attributes = String::new();
    for index in 0..1_025 {
        write!(attributes, " a{index}=\"x\"").unwrap();
    }
    let xml = format!(r#"<Root xmlns="urn:openbim:gaeb:test"{attributes}/>"#);
    let report = schema.validate(xml.as_bytes()).unwrap();
    assert!(report.has_code("XSD-XML-PARSE"));
    assert!(report.diagnostics()[0]
        .message()
        .contains("attribute count exceeds 1024"));

    let repeated_invalid_values = "<Value>not-an-int</Value>".repeat(5_000);
    let xml = format!(r#"<Root xmlns="urn:openbim:gaeb:test">{repeated_invalid_values}</Root>"#);
    let report = schema.validate(xml.as_bytes()).unwrap();
    assert!(report.has_code("XSD-DIAGNOSTICS-TRUNCATED"));
    assert!(report.diagnostics().len() <= 4_097);
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

#[test]
fn schema_loading_fails_closed_when_include_namespace_mismatches() {
    let directory = tempfile::tempdir().unwrap();
    let schema = directory.path().join("root.xsd");
    std::fs::write(
        &schema,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:root"><xs:include schemaLocation="included.xsd"/></xs:schema>"#,
    )
    .unwrap();
    std::fs::write(
        directory.path().join("included.xsd"),
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:other"/>"#,
    )
    .unwrap();
    assert!(XsdSchema::from_file(schema).is_err());
}

#[test]
fn schema_loading_fails_closed_for_namespace_only_imports() {
    let directory = tempfile::tempdir().unwrap();
    let schema = directory.path().join("root.xsd");
    std::fs::write(
        &schema,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:import namespace="urn:missing"/><xs:element name="Root" type="xs:string"/></xs:schema>"#,
    )
    .unwrap();
    assert!(XsdSchema::from_file(schema).is_err());
}

#[test]
fn schema_loading_confines_directives_to_the_root_directory() {
    let parent = tempfile::tempdir().unwrap();
    let directory = parent.path().join("schemas");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(
        parent.path().join("outside.xsd"),
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"/>"#,
    )
    .unwrap();
    let schema = directory.join("root.xsd");
    std::fs::write(
        &schema,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:include schemaLocation="../outside.xsd"/></xs:schema>"#,
    )
    .unwrap();
    let error = match XsdSchema::from_file(schema) {
        Ok(_) => panic!("schema escape was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("non-local path component"), "{error}");
}

#[cfg(unix)]
#[test]
fn schema_loading_rejects_symlinked_directives() {
    use std::os::unix::fs::symlink;

    let parent = tempfile::tempdir().unwrap();
    let directory = parent.path().join("schemas");
    std::fs::create_dir(&directory).unwrap();
    let outside = parent.path().join("outside.xsd");
    std::fs::write(
        &outside,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"/>"#,
    )
    .unwrap();
    symlink(&outside, directory.join("linked.xsd")).unwrap();
    let schema = directory.join("root.xsd");
    std::fs::write(
        &schema,
        br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:include schemaLocation="linked.xsd"/></xs:schema>"#,
    )
    .unwrap();
    let error = match XsdSchema::from_file(schema) {
        Ok(_) => panic!("symlinked schema escape was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("symbolic link"), "{error}");
}

#[test]
fn schema_loading_bounds_graph_depth() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..=65 {
        let include = if index < 65 {
            format!(r#"<xs:include schemaLocation="{}.xsd"/>"#, index + 1)
        } else {
            String::new()
        };
        std::fs::write(
            directory.path().join(format!("{index}.xsd")),
            format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{include}</xs:schema>"#
            ),
        )
        .unwrap();
    }
    let error = match XsdSchema::from_file(directory.path().join("0.xsd")) {
        Ok(_) => panic!("over-deep schema graph was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("depth exceeds 64"), "{error}");
}

#[test]
fn schema_loading_bounds_graph_cardinality() {
    let directory = tempfile::tempdir().unwrap();
    let mut includes = String::new();
    for index in 0..256 {
        writeln!(includes, r#"<xs:include schemaLocation="{index}.xsd"/>"#).unwrap();
        std::fs::write(
            directory.path().join(format!("{index}.xsd")),
            br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"/>"#,
        )
        .unwrap();
    }
    let root = directory.path().join("root.xsd");
    std::fs::write(
        &root,
        format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{includes}</xs:schema>"#),
    )
    .unwrap();
    let error = match XsdSchema::from_file(root) {
        Ok(_) => panic!("over-wide schema graph was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("document count exceeds 256"), "{error}");
}

#[test]
fn schema_loading_bounds_total_graph_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root.xsd");
    let file = std::fs::File::create(&root).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    let error = match XsdSchema::from_file(root) {
        Ok(_) => panic!("over-budget schema graph was accepted"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("graph bytes exceed 8388608"), "{error}");
}
