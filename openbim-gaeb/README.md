# openbim-gaeb

Pure-Rust, lossless GAEB DA XML recognition, inspection, focused editing, and
caller-provided XSD validation.

## Capability status

### Lossless core

- exact namespace recognition across the audited GAEB `3.1`–`3.4` schema
  corpus, including both legacy `3.1` namespaces;
- namespace-resolved GAEB elements plus structurally scoped `<Version>` and
  `<DP>` evidence with mismatch and duplicate-declaration diagnostics;
- byte-identical unchanged round trips, including BOM, comments, prefixes,
  whitespace, and unknown extensions;
- explicit rejection above 256 nested XML elements to bound parser state for
  untrusted documents;
- common BoQ summaries and atomic `<Qty>` edits by unique item ID.

### XSD validation

`XsdSchema` compiles a caller-provided schema graph once and validates multiple
instances with `xsd-schema`. `GaebSchemaSet` loads the exact schema roots in
`support-matrix.csv` and dispatches in-memory documents by version, version date,
exact phase, and namespace—never by filename.

The required nested-redefine fixes currently come from immutable `xsd-schema`
revision `53de66ccb075246a67e5986742cdcdb5deb81267`. Package verification patches
extracted artifacts to that same revision. All workspace packages set
`publish = false` until the equivalent upstream registry release is available.

Strict schema-derivation validation is the default. Callers with a trusted,
independently verified official corpus may explicitly disable only that schema
consistency stage:

```rust
use openbim_gaeb::{XsdLoadOptions, XsdSchema};

let schema = XsdSchema::from_file_with_options(
    "/path/to/caller-provided/GAEB_DA_XML.xsd",
    XsdLoadOptions { validate_schema_derivations: false },
)?;
let report = schema.validate(xml_bytes)?;
assert!(report.is_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The opt-out does not skip schema graph loading, `xs:include`/`xs:redefine`
resolution, reference resolution, or instance validation. Official GAEB schemas
and fixtures are not redistributed by this crate.

### Support evidence

- `schema-support-matrix.json`: 87 exact semantic rows, 93 snapshot dispatch
  edges, and 80 physical roots derived from the 126-file official corpus;
- `support-matrix.csv`: the eight GAEB 3.1/3.2 profiles with official instance
  fixtures and proven XSD dispatch;
- typed support is narrower: the opt-in sibling crate
  `openbim-gaeb-bindings` claims only GAEB 3.1 X81, X83, and X86. Each row is
  proven by parse → typed write → typed reparse, exact official-XSD validation,
  and exact decimal-value retention. X84 and GAEB 3.2 are deliberately not
  claimed because current generated bindings fail their executable gates.

### Business validation

`BusinessValidator` exposes 18 checks split into three evidence-backed
`GAEB-BR-*` conformance errors and fifteen advisory `GAEB-LINT-*` warnings.
XSD validation remains the authoritative structural gate. Per-check evidence,
known scope gaps, and promotion criteria are documented in
[business-rule evidence audit](https://github.com/openbimrs/gaeb/blob/master/docs/business-rules.md).

## Example

```rust
use openbim_gaeb::Document;

let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3">
  <GAEBInfo><Version>3.3</Version></GAEBInfo>
  <Award><DP>86</DP><BoQ><BoQBody><Itemlist>
    <Item ID="i-1"><Qty>1.000</Qty></Item>
  </Itemlist></BoQBody></BoQ></Award>
</GAEB>"#;
let mut document = Document::parse(xml)?;
document.set_item_quantity("i-1", "2.500")?;
assert!(String::from_utf8_lossy(document.as_bytes()).contains("<Qty>2.500</Qty>"));
# Ok::<(), openbim_gaeb::Error>(())
```

The canonical repository is <https://github.com/openbimrs/gaeb>.

## License

MIT
