use std::{env, fs, path::Path};

use openbim_gaeb::{support::SUPPORT_MATRIX, Document, GaebSchemaSet};
use openbim_gaeb_bindings::{Error, TypedDocument};
use quick_xml::{events::Event, Reader};
use rust_decimal::Decimal;
use std::str::FromStr;

fn numeric_leaf_values(xml: &[u8]) -> Vec<(String, Decimal)> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut path = Vec::new();
    let mut values = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer).unwrap() {
            Event::Start(start) => {
                path.push(String::from_utf8_lossy(start.name().as_ref()).into_owned());
            }
            Event::Empty(_) => {}
            Event::End(_) => {
                path.pop();
            }
            Event::Text(text) => {
                let value = String::from_utf8_lossy(text.as_ref());
                if let Ok(decimal) = Decimal::from_str(value.trim()) {
                    values.push((path.join("/"), decimal));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    values
}

#[test]
#[ignore = "requires caller-provided official GAEB fixture corpus"]
fn every_claimed_typed_row_parse_writes_and_reparses() {
    let root = env::var_os("GAEB_OFFICIAL_FIXTURES")
        .expect("set GAEB_OFFICIAL_FIXTURES to the unmodified fixture corpus");
    let root = Path::new(&root);
    let schema_root = env::var_os("GAEB_OFFICIAL_SCHEMA_ROOT")
        .expect("set GAEB_OFFICIAL_SCHEMA_ROOT to the unmodified schema corpus");
    let schemas = GaebSchemaSet::load_official(Path::new(&schema_root)).unwrap();
    let claimed: Vec<_> = SUPPORT_MATRIX
        .iter()
        .filter(|entry| entry.typed_module.is_some())
        .collect();
    assert_eq!(claimed.len(), 3, "typed claims must stay evidence-bounded");

    for entry in claimed {
        let bytes = fs::read(root.join(entry.fixture)).unwrap();
        let document = Document::parse(&bytes).unwrap();
        let typed = TypedDocument::parse(&document).unwrap();
        let serialized = typed.to_xml().unwrap();
        assert_eq!(
            numeric_leaf_values(&bytes),
            numeric_leaf_values(&serialized),
            "typed output changed an exact decimal value for {}",
            entry.fixture
        );
        let reparsed = Document::parse(&serialized).unwrap();
        TypedDocument::parse(&reparsed).unwrap();
        let report = schemas.validate_document(&reparsed).unwrap();
        assert!(
            report.is_valid(),
            "typed output for {} is XSD-invalid: {:#?}",
            entry.fixture,
            report.diagnostics()
        );
    }
}

#[test]
#[ignore = "requires caller-provided official GAEB fixture corpus"]
fn unproven_x84_is_not_claimed_as_typed() {
    let root = env::var_os("GAEB_OFFICIAL_FIXTURES")
        .expect("set GAEB_OFFICIAL_FIXTURES to the unmodified fixture corpus");
    let entry = SUPPORT_MATRIX
        .iter()
        .find(|entry| entry.fixture.ends_with("X84"))
        .unwrap();
    assert!(entry.typed_module.is_none());
    let bytes = fs::read(Path::new(&root).join(entry.fixture)).unwrap();
    let document = Document::parse(&bytes).unwrap();
    assert!(matches!(
        TypedDocument::parse(&document),
        Err(Error::UntypedProfile)
    ));
}
