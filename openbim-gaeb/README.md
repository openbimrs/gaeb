# openbim-gaeb

Pure-Rust, lossless GAEB DA XML recognition, inspection, and focused editing.

## Implemented

- exact namespace recognition from the pinned official `3.1` through `3.4`
  schema matrix, including both legacy `3.1` namespaces;
- namespace-resolved GAEB elements plus structurally scoped `<Version>` and `<DP>`
  evidence with mismatch and duplicate-declaration diagnostics;
- byte-identical unchanged round trips, including BOM, comments, prefixes,
  whitespace, and unknown extensions;
- common BoQ summaries for schema-positioned `Itemlist/Item` elements, using only
  the direct item description subtree;
- atomic `<Qty>` edits by unique, non-empty item ID when the value has one safe
  text or CDATA range; comments make values read-only, while nested markup or
  duplicate quantities are treated as semantically ambiguous and not exposed.

The crate does **not** claim full XSD bindings, XSD validation, or business-rule
validation.

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
