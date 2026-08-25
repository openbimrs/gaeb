use std::{env, fs, process::ExitCode};

use openbim_gaeb::Document;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: cargo run --example inspect -- <GAEB-XML-file>");
        return ExitCode::from(2);
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    let document = match Document::parse(bytes) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("cannot parse {}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    let metadata = document.metadata();
    println!("namespace: {}", metadata.namespace);
    println!(
        "version: {}",
        metadata
            .version
            .map_or("unknown", |version| version.as_str())
    );
    println!(
        "phase: {}",
        metadata.phase.map_or("unknown", |phase| phase.as_code())
    );
    println!("items: {}", document.items().len());
    println!("diagnostics: {}", document.diagnostics().len());
    for diagnostic in document.diagnostics() {
        println!("- {:?}: {}", diagnostic.kind, diagnostic.message);
    }
    ExitCode::SUCCESS
}
