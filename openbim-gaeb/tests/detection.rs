use openbim_gaeb::{DiagnosticKind, Document, Error, ExchangePhase, GaebVersion};

#[test]
fn detects_stable_33_from_consistent_namespace_header_and_phase() {
    let xml = br#"<?xml version="1.0"?>
<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">
  <GAEBInfo><Version>3.3</Version><VersDate>2023-01</VersDate><Date>2026-08-25</Date><Time>10:15:00</Time><ProgSystem>OpenBIM.rs</ProgSystem><ProgName>gaeb</ProgName></GAEBInfo>
  <Award><DP>83</DP></Award>
</GAEB>"#;

    let document = Document::parse(xml).unwrap();
    let metadata = document.metadata();
    assert_eq!(
        metadata.namespace,
        "http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"
    );
    assert_eq!(metadata.version, Some(GaebVersion::V3_3));
    assert_eq!(metadata.version_date.as_deref(), Some("2023-01"));
    assert_eq!(metadata.phase, Some(ExchangePhase::X83));
    assert_eq!(metadata.program_system.as_deref(), Some("OpenBIM.rs"));
    assert_eq!(metadata.program_name.as_deref(), Some("gaeb"));
    assert!(document.diagnostics().is_empty());
}

#[test]
fn detects_legacy_31_where_namespace_does_not_encode_phase() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/200407"><GAEBInfo><Version>3.1</Version><VersDate>2007-06</VersDate></GAEBInfo><Award><DP>81</DP></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();
    assert_eq!(document.metadata().version, Some(GaebVersion::V3_1));
    assert_eq!(document.metadata().phase, Some(ExchangePhase::X81));
}

#[test]
fn marks_34_as_beta() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA81/3.4"><GAEBInfo><Version>3.4</Version><VersDate>2026-03</VersDate></GAEBInfo><Award><DP>81</DP></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();
    assert_eq!(document.metadata().version, Some(GaebVersion::V3_4Beta));
    assert!(document.metadata().version.unwrap().is_beta());
}

#[test]
fn surfaces_namespace_and_payload_disagreement() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA84/3.3"><GAEBInfo><Version>3.2</Version></GAEBInfo><Award><DP>83</DP></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();
    let kinds: Vec<_> = document.diagnostics().iter().map(|d| d.kind).collect();
    assert!(kinds.contains(&DiagnosticKind::VersionMismatch));
    assert!(kinds.contains(&DiagnosticKind::PhaseMismatch));
}

#[test]
fn coarse_50_namespace_accepts_specific_50_1_phase() {
    let document = Document::parse(
        br#"<g:GAEB xmlns:g="http://www.gaeb.de/GAEB_DA_XML/DA50/3.3"><g:GAEBInfo><g:Version>3.3</g:Version></g:GAEBInfo><g:ElementalCosting><g:DP>50.1</g:DP></g:ElementalCosting></g:GAEB>"#,
    )
    .unwrap();
    assert_eq!(document.metadata().version, Some(GaebVersion::V3_3));
    assert_eq!(document.metadata().namespace_phase, None);
    assert_eq!(document.metadata().phase, Some(ExchangePhase::X50_1));
    assert!(document.diagnostics().is_empty());
}

#[test]
fn duplicate_version_and_phase_declarations_are_explicitly_diagnosed() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><GAEBInfo><Version>3.3</Version><Version>3.4</Version></GAEBInfo><Award><DP>83</DP><DP>84</DP></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();

    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DiagnosticKind::DuplicateVersionDeclaration));
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DiagnosticKind::DuplicatePhaseDeclaration));
    assert_eq!(document.metadata().version_text.as_deref(), Some("3.3"));
    assert_eq!(document.metadata().phase_code.as_deref(), Some("83"));
}

#[test]
fn only_schema_defined_top_level_phase_locations_are_interpreted() {
    let misplaced = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/200407"><Anything><DP>83</DP></Anything></GAEB>"#,
    )
    .unwrap();
    assert_eq!(misplaced.metadata().phase, None);

    let x61 = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA61/3.3"><GAEBInfo><DP>61</DP></GAEBInfo></GAEB>"#,
    )
    .unwrap();
    assert_eq!(x61.metadata().phase, Some(ExchangePhase::X61));
}

#[test]
fn empty_version_and_phase_elements_still_count_as_declarations() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><GAEBInfo><Version/><Version>3.3</Version></GAEBInfo><Award><DP/><DP>83</DP></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();

    assert_eq!(document.metadata().version_text, None);
    assert_eq!(document.metadata().phase_code, None);
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DiagnosticKind::DuplicateVersionDeclaration));
    assert!(document
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DiagnosticKind::DuplicatePhaseDeclaration));
}

#[test]
fn detects_phase_31_under_quantity_determination() {
    let document = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA31/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><QtyDeterm><DP>31</DP></QtyDeterm></GAEB>"#,
    )
    .unwrap();
    assert_eq!(document.metadata().phase, Some(ExchangePhase::X31));
}

#[test]
fn phase_declarations_require_the_product_specific_gaeb_parent() {
    for xml in [
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><Invoice><DP>86</DP></Invoice></GAEB>"#.as_slice(),
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA31/3.3"><Award><DP>31</DP></Award></GAEB>"#,
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA31/3.3"><QtyDetermination><DP>31</DP></QtyDetermination></GAEB>"#,
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:f="urn:vendor"><f:Award><DP>86</DP></f:Award></GAEB>"#,
    ] {
        let document = Document::parse(xml).unwrap();
        assert_eq!(document.metadata().declared_phase, None);
        assert_eq!(document.metadata().phase_code, None);
    }

    for (namespace, parent, code, phase) in [
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA31/3.3",
            "QtyDeterm",
            "31",
            ExchangePhase::X31,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA50/3.3",
            "ElementalCosting",
            "50.1",
            ExchangePhase::X50_1,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA61/3.3",
            "GAEBInfo",
            "61",
            ExchangePhase::X61,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA84P/3.3",
            "SC_Evaluation",
            "84P",
            ExchangePhase::X84P,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA89/3.3",
            "Invoice",
            "89",
            ExchangePhase::X89,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA93/3.3",
            "Order",
            "93",
            ExchangePhase::X93,
        ),
    ] {
        let xml =
            format!(r#"<GAEB xmlns="{namespace}"><{parent}><DP>{code}</DP></{parent}></GAEB>"#);
        let document = Document::parse(xml).unwrap();
        assert_eq!(document.metadata().declared_phase, Some(phase));
    }
}

#[test]
fn resolves_character_references_in_namespace_declarations() {
    let document =
        Document::parse(br#"<g:GAEB xmlns:g="http://www.gaeb.de/GAEB_DA_XML/DA83/&#x33;.3"/>"#)
            .unwrap();
    assert_eq!(
        document.metadata().namespace,
        "http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"
    );
    assert_eq!(document.metadata().version, Some(GaebVersion::V3_3));
}

#[test]
fn recognizes_only_namespaces_present_in_the_official_schema_snapshots() {
    let cases = [
        ("http://www.gaeb.de/GAEB_DA_XML/200407", GaebVersion::V3_1),
        ("http://www.gaeb.de/GAEB_DA_XML/200706", GaebVersion::V3_1),
        ("http://www.gaeb.de/GAEB_DA_XML/DA31/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA52/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA80/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA81/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA82/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA83/3.2", GaebVersion::V3_2),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA83Z/3.2",
            GaebVersion::V3_2,
        ),
        ("http://www.gaeb.de/GAEB_DA_XML/DA84/3.2", GaebVersion::V3_2),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA84Z/3.2",
            GaebVersion::V3_2,
        ),
        ("http://www.gaeb.de/GAEB_DA_XML/DA85/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA86/3.2", GaebVersion::V3_2),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA86ZE/3.2",
            GaebVersion::V3_2,
        ),
        (
            "http://www.gaeb.de/GAEB_DA_XML/DA86ZR/3.2",
            GaebVersion::V3_2,
        ),
        ("http://www.gaeb.de/GAEB_DA_XML/DA87/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA89/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA93/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA94/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA96/3.2", GaebVersion::V3_2),
        ("http://www.gaeb.de/GAEB_DA_XML/DA97/3.2", GaebVersion::V3_2),
    ];
    for (namespace, expected) in cases {
        let xml = format!(r#"<GAEB xmlns="{namespace}"/>"#);
        let document = Document::parse(xml.as_bytes())
            .unwrap_or_else(|error| panic!("official namespace {namespace} was rejected: {error}"));
        assert_eq!(document.metadata().namespace_version, Some(expected));
    }

    for (version, expected, phases) in [
        (
            "3.3",
            GaebVersion::V3_3,
            "31,50,51,52,61,80,81,82,83,83Z,84,84P,84Z,85,86,86ZE,86ZR,87,89,89B,93,94,96,97,98,99",
        ),
        (
            "3.4",
            GaebVersion::V3_4Beta,
            "31,50,51,52,61,80,81,82,83,83Z,84,84P,84Z,85,86,86ZE,86ZR,87,89,89B,93,94,96,97,98,99",
        ),
    ] {
        for phase in phases.split(',') {
            let namespace = format!("http://www.gaeb.de/GAEB_DA_XML/DA{phase}/{version}");
            let xml = format!(r#"<GAEB xmlns="{namespace}"/>"#);
            let document = Document::parse(xml.as_bytes()).unwrap_or_else(|error| {
                panic!("official namespace {namespace} was rejected: {error}")
            });
            assert_eq!(document.metadata().namespace_version, Some(expected));
        }
    }
}

#[test]
fn rejects_nonexistent_namespace_version_phase_products() {
    for namespace in [
        "http://www.gaeb.de/GAEB_DA_XML/DA50.1/3.3",
        "http://www.gaeb.de/GAEB_DA_XML/DA88/3.3",
        "http://www.gaeb.de/GAEB_DA_XML/DA99/3.2",
        "http://www.gaeb.de/GAEB_DA_XML/DA81/3.1",
    ] {
        let xml = format!(r#"<GAEB xmlns="{namespace}"/>"#);
        assert!(matches!(Document::parse(xml), Err(Error::NotGaeb)));
    }
}

#[test]
fn accepts_official_31_order_namespace_and_ambiguous_84_phase() {
    let order = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/200706"><GAEBInfo><Version>3.1</Version></GAEBInfo><Order><DP>93</DP></Order></GAEB>"#,
    )
    .unwrap();
    assert_eq!(order.metadata().phase, Some(ExchangePhase::X93));
    assert!(order.diagnostics().is_empty());

    let x84z = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA84/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>84Z</DP></Award></GAEB>"#,
    )
    .unwrap();
    assert_eq!(x84z.metadata().namespace_phase, None);
    assert_eq!(x84z.metadata().phase, Some(ExchangePhase::X84Z));
    assert!(!x84z
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DiagnosticKind::PhaseMismatch));
}
