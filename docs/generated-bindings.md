# Generated binding provenance

`openbim-gaeb-bindings/src/generated/v3_1_2007_11.rs` is the only generated binding family shipped in this workspace. It is opt-in and separate from the byte-lossless `openbim-gaeb` core.

## Inputs

The source snapshot is the caller-provided `gaeb-da-xml-3.1-2007-11` directory. Official schema bytes are not redistributed. Generation consumed these five files as one schema graph:

| File | SHA-256 |
|---|---|
| `GAEB_DA_XML.xsd` | `8db4edb24c6f5390650b527abe38d82e32dea2011e7f904234cf073fd6693817` |
| `GAEB_DA_XML_84.xsd` | `be4e7389e38386d1d0203b60180967ed52f2b540c05ca80d76cd4b200fcd0927` |
| `GAEB_DA_XML_Order.xsd` | `464f56ba306baa07d4add8ffb44fbbec2ccd516d4303980b62a7fe1b339eb0fa` |
| `xml.xsd` | `068d7ed95badc1ccf1be78b47e97a3ad9f7d10e17d3d3d684606e62a85de20ed` |
| `xmldsig-core-schema.xsd` | `51f45a96104c905697c5a708357c10e4311c04ff048dd78eeb5a264f835d0614` |

Verify caller-provided inputs before regeneration:

```sh
sha256sum GAEB_DA_XML.xsd GAEB_DA_XML_84.xsd \
  GAEB_DA_XML_Order.xsd xml.xsd xmldsig-core-schema.xsd
```

## Generator configuration

The raw module was emitted with the `xsd-parser 1.5.2` API using:

- all five schemas above in `Config::parser.schemas`;
- `with_element_postfix("Element")`;
- `GeneratorFlags::FLATTEN_STRUCT_CONTENT`;
- `with_quick_xml()`;
- one module rather than namespace modules.

The unformatted raw output SHA-256 was `f343208a9e554aeae78a2916730cbe354adc34d62fd9fa3c27fa001838b29e52`.

## Reviewed post-generation corrections

The checked-in artifact deliberately differs from raw generator output in two localized ways:

1. the serializer writes the expanded XML attribute as `xml:space`, matching what its generated deserializer accepts and what the official XSD requires;
2. XSD decimal aliases use the local `ExactDecimal` wrapper over `rust_decimal::Decimal`, not binary `f64`. The wrapper implements the generated runtime's byte serialization/deserialization contracts without losing commercial decimal values.

These corrections are executable release requirements. After Rust 1.85 formatting,
the checked-in corrected artifact SHA-256 is
`6d12926d7dd0ce539bce25b831e803217acc593ee399b7a16e8977267dfe78ec`.

- `official_roundtrip` parses, writes, reparses, XSD-validates, and compares all numeric leaf values for every claimed official profile;
- `exact_decimal` proves that GAEB decimal aliases preserve a value beyond binary64 integer precision;
- mutating the GAEB aliases back to `f64` makes the focused decimal gate fail.

## Supported boundary

Only GAEB 3.1 X81, X83, and X86 are claimed. The official X84 fixture does not decode through this generated family and remains unclaimed. GAEB 3.2 generated families do not compile and are not shipped. See `openbim-gaeb/schema-support-matrix.json` for the machine-readable claim boundary.
