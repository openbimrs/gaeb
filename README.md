# OpenBIM.rs GAEB

[![CI](https://github.com/openbimrs/gaeb/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/gaeb/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/openbim-gaeb.svg)](https://crates.io/crates/openbim-gaeb)
[![docs.rs](https://docs.rs/openbim-gaeb/badge.svg)](https://docs.rs/openbim-gaeb)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://www.rust-lang.org)

Pure-Rust, lossless tools for working with GAEB DA XML bills of quantities.

This repository is the canonical GAEB family in
[OpenBIM.rs](https://github.com/openbimrs/openbim). The integration repository
pins verified revisions under `packages/gaeb`.

## Status

The initial implementation reads GAEB DA XML into a lossless owned document,
cross-checks version and exchange-phase evidence, extracts common BoQ item
fields, and supports atomic quantity edits without regenerating unrelated XML.

| Capability | Status |
| --- | --- |
| GAEB DA XML 3.1 recognition | Implemented and tested |
| GAEB DA XML 3.2 recognition | Implemented and official-example tested |
| GAEB DA XML 3.3 recognition | Implemented and synthetic-fixture tested |
| GAEB DA XML 3.4 beta recognition | Implemented; explicitly marked beta |
| Namespace / `<Version>` / `<DP>` conflict diagnostics | Implemented |
| Byte-identical unchanged round trip | Implemented |
| Common BoQ item view (`ID`, number, quantity, unit, prices, description) | Implemented |
| Lossless quantity edit by item ID | Implemented |
| Full typed binding for every exchange phase | Not implemented |
| XSD validation | Not implemented |
| Business-rule validation | Not implemented |

No full-schema or business-validation capability should be inferred from the
lossless reader.

## Crates

| Crate | Purpose |
| --- | --- |
| [`openbim-gaeb`](openbim-gaeb/) | Canonical implementation and public types |
| [`gaeb`](gaeb/) | Short-name compatibility re-export |

## Install

```bash
cargo add gaeb
```

Until the short alias is published, use the canonical package:

```bash
cargo add openbim-gaeb
```

```rust
use openbim_gaeb::{Document, ExchangePhase, GaebVersion};

let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">
  <GAEBInfo><Version>3.3</Version><VersDate>2023-01</VersDate></GAEBInfo>
  <Award><DP>83</DP></Award>
</GAEB>"#;
let document = Document::parse(xml)?;
assert_eq!(document.metadata().version, Some(GaebVersion::V3_3));
assert_eq!(document.metadata().phase, Some(ExchangePhase::X83));
assert_eq!(document.as_bytes(), xml);
# Ok::<(), openbim_gaeb::Error>(())
```

## XML boundary

`openbim-codec-xml` owns only format-neutral BOM stripping and content sniffing.
GAEB owns streaming element interpretation, exchange phases, BoQ semantics,
diagnostics, and edits. This keeps GAEB quirks out of the shared codec while
avoiding duplicate container recognition.

See [`docs/architecture.md`](docs/architecture.md).

## Official references

Official GAEB schemas and examples are useful verification oracles, but their
public redistribution license is not explicit. They are therefore downloaded
locally and checksum-verified rather than committed to this MIT repository:

```bash
./scripts/fetch-official-references.py
GAEB_OFFICIAL_EXAMPLES="$PWD/references/examples" \
  cargo test -p openbim-gaeb --test official_corpus -- --ignored
```

The pinned URLs, archive hashes, and extracted-file hashes are recorded in
[`references/SOURCE-MANIFEST.json`](references/SOURCE-MANIFEST.json).

## Development

Requires Rust `1.85` or newer.

```bash
./scripts/gate.sh
```

The gate checks formatting, all targets, tests, Clippy, rustdoc, and package
contents from actual command exit codes.

## License

MIT — see [`LICENSE`](LICENSE).
