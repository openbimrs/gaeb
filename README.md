# OpenBIM.rs GAEB

[![CI](https://github.com/openbimrs/gaeb/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/gaeb/actions/workflows/ci.yml)
[![openbim-gaeb](https://img.shields.io/crates/v/openbim-gaeb.svg)](https://crates.io/crates/openbim-gaeb)
[![gaeb](https://img.shields.io/crates/v/gaeb.svg)](https://crates.io/crates/gaeb)
[![docs.rs](https://docs.rs/openbim-gaeb/badge.svg)](https://docs.rs/openbim-gaeb)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://www.rust-lang.org)

Pure-Rust, lossless tools for working with GAEB DA XML bills of quantities.

This repository is the canonical GAEB family in
[OpenBIM.rs](https://github.com/openbimrs/openbim). The integration repository
pins verified revisions under `packages/gaeb`.

## Status

The initial implementation reads GAEB DA XML into a lossless owned document,
cross-checks version and exchange-phase evidence, extracts common BoQ item
fields, and supports atomic quantity edits when the complete value has one
safely replaceable XML text or CDATA range. Mixed-content values fail closed.

| Capability | Status |
| --- | --- |
| Exact official namespace/version matrix, including both 3.1 namespaces | Implemented and tested |
| GAEB DA XML 3.2 recognition | Implemented and official-example tested |
| GAEB DA XML 3.3 recognition | Implemented and synthetic-fixture tested; some phases are beta |
| GAEB DA XML 3.4 beta recognition | Implemented; generation explicitly marked beta |
| Namespace-resolved `<Version>` / `<DP>` conflict diagnostics | Implemented |
| Byte-identical unchanged round trip | Implemented |
| Common BoQ item view (`ID`, number, quantity, unit, prices, direct description) | Implemented for schema-positioned `Itemlist/Item` |
| Safe quantity edit by unique, non-empty item ID | Implemented for one complete scalar range; nested/repeated values are not exposed |
| Typed bindings | Opt-in sibling crate; fixture/XSD/decimal-round-trip proven for GAEB 3.1 X81, X83, and X86 only |
| Caller-provided XSD validation | Implemented; eight official fixture profiles and all 126 official XSDs verified locally |
| Business-rule validation | 18 checks with pairwise executable coverage; 3 narrowly scoped conformance errors and 15 explicitly advisory `GAEB-LINT` warnings; see [`docs/business-rules.md`](docs/business-rules.md) |

Typed or business support must not be inferred from namespace recognition alone.

## Crates

| Crate | Purpose |
| --- | --- |
| [`openbim-gaeb`](openbim-gaeb/) | Lossless core, exact support manifests, XSD validation, and business checks |
| [`openbim-gaeb-bindings`](openbim-gaeb-bindings/) | Opt-in generated typed bindings for fixture-proven profiles |
| [`gaeb`](gaeb/) | Short-name compatibility re-export |

## Install

The commands below install the last registry release. The unreleased XSD and
typed-binding work in this repository is source-only while its required
`xsd-schema` fix remains Git-pinned; all three manifests deliberately set
`publish = false`.

Use either package name:

```bash
cargo add openbim-gaeb
# or
cargo add gaeb
```

Do not depend on both directly. `gaeb` already brings in the exact canonical
`openbim-gaeb` version and re-exports it without defining independent types.

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

The crate uses dedicated upstream XML infrastructure (`quick-xml`) directly,
validates namespace URI references with `iri-string`, and owns its small
BOM/content-detection policy beside the parser. GAEB-specific
strict parsing policy, namespace resolution, exchange phases, BoQ semantics,
diagnostics, and lossless edits remain here; no project-owned generic XML/ZIP
abstraction leaks policy across different openBIM formats.

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
./scripts/mutation-probes.py
```

The main gate checks formatting, all targets, tests, Clippy, rustdoc, and package
contents from actual command exit codes. The mutation gate independently proves
that version conflicts, decimal validation, BOM preservation, namespace
isolation, attribute-prefix validation, exact namespace matrices, schema-scoped
items/descriptions/phases, empty/repeated evidence stability, and fragmented or nested
quantity handling remain enforced; CI runs both.

## License

MIT — see [`LICENSE`](LICENSE).
