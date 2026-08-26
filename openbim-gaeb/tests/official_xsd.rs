use openbim_gaeb::{Document, GaebSchemaSet, XsdLoadOptions, XsdSchema};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect_xsds(path: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            collect_xsds(&entry, out);
        } else if entry.extension().and_then(|value| value.to_str()) == Some("xsd") {
            out.push(entry);
        }
    }
}

#[test]
#[ignore = "requires caller-provided official GAEB schemas"]
fn every_official_xsd_compiles_without_modification() {
    let root = PathBuf::from(env::var_os("GAEB_OFFICIAL_SCHEMA_ROOT").expect(
        "set GAEB_OFFICIAL_SCHEMA_ROOT to references/specs from fetch-official-references.py",
    ));
    let mut schemas = Vec::new();
    collect_xsds(&root, &mut schemas);
    assert_eq!(schemas.len(), 126, "official snapshot inventory changed");

    let options = XsdLoadOptions {
        validate_schema_derivations: false,
    };
    let mut failures = Vec::new();
    for schema in &schemas {
        if let Err(error) = XsdSchema::from_file_with_options(schema, options) {
            failures.push(format!("{}: {error}", schema.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "requires caller-provided official GAEB schemas and fixtures"]
fn every_fixture_backed_profile_validates_with_its_exact_schema_root() {
    let schema_root = PathBuf::from(
        env::var_os("GAEB_OFFICIAL_SCHEMA_ROOT")
            .expect("set GAEB_OFFICIAL_SCHEMA_ROOT to references/specs"),
    );
    let fixture_root = PathBuf::from(
        env::var_os("GAEB_OFFICIAL_FIXTURES")
            .expect("set GAEB_OFFICIAL_FIXTURES to the unmodified fixture corpus"),
    );
    let schemas = GaebSchemaSet::load_official(&schema_root).unwrap();
    for entry in openbim_gaeb::support::SUPPORT_MATRIX {
        let bytes = fs::read(fixture_root.join(entry.fixture)).unwrap();
        let document = Document::parse(&bytes).unwrap();
        let candidates: Vec<_> =
            openbim_gaeb::support::candidates_for_document(&document).collect();
        assert_eq!(
            candidates,
            vec![entry],
            "dispatch drift for {}",
            entry.fixture
        );
        let report = schemas.validate_document(&document).unwrap();
        assert!(
            report.is_valid(),
            "{}: {:?}",
            entry.fixture,
            report.diagnostics()
        );
    }
}
