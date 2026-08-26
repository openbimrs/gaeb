# Typed bindings crate

- `src/generated/` contains generated quick-XML bindings grouped by exact schema snapshot/family.
- `src/lib.rs` owns exact support-matrix dispatch and parse/write errors.
- Do not expose or claim a generated module until an official fixture passes parse → write → reparse under Rust 1.85.
- Do not commit official GAEB XSD or fixture bytes.
