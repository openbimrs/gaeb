use openbim_gaeb::{DiagnosticKind, Document, ExchangePhase, GaebVersion};

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
        br#"<g:GAEB xmlns:g="http://www.gaeb.de/GAEB_DA_XML/DA50/3.3"><g:GAEBInfo><g:Version>3.3</g:Version></g:GAEBInfo><g:Award><g:DP>50.1</g:DP></g:Award></g:GAEB>"#,
    )
    .unwrap();
    assert_eq!(document.metadata().version, Some(GaebVersion::V3_3));
    assert_eq!(document.metadata().namespace_phase, None);
    assert_eq!(document.metadata().phase, Some(ExchangePhase::X50_1));
    assert!(document.diagnostics().is_empty());
}

#[test]
fn detects_phase_31_under_quantity_determination() {
    let document = Document::parse(
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA31/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><QtyDeterm><DP>31</DP></QtyDeterm></GAEB>"#,
    )
    .unwrap();
    assert_eq!(document.metadata().phase, Some(ExchangePhase::X31));
}
