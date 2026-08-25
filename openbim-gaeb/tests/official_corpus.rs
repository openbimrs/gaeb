use openbim_gaeb::Document;
use std::{env, fs, path::Path};

fn collect_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
#[ignore = "requires locally downloaded official GAEB examples"]
fn parses_every_official_example_losslessly() {
    let root = env::var_os("GAEB_OFFICIAL_EXAMPLES").expect("set GAEB_OFFICIAL_EXAMPLES");
    let mut files = Vec::new();
    collect_files(Path::new(&root), &mut files);
    assert!(!files.is_empty(), "official corpus is empty");

    for path in files {
        let bytes = fs::read(&path).unwrap();
        let document =
            Document::parse(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(document.as_bytes(), bytes, "{}", path.display());
        assert!(document.metadata().phase.is_some(), "{}", path.display());
        assert!(!document.items().is_empty(), "{}", path.display());
        assert!(
            document.diagnostics().is_empty(),
            "{}: {:?}",
            path.display(),
            document.diagnostics()
        );
    }
}
