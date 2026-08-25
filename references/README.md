# Official GAEB DA XML references

This directory describes unmodified GAEB DA XML schemas and a compact set of
official example files from the [GAEB data-exchange download page][source]. The
payload directories are intentionally ignored by Git because the official site
does not state an explicit public redistribution license. Restore and verify
them locally with:

```bash
./scripts/fetch-official-references.py
```

They are reference/test data, not authored OpenBIM.rs source.

## Schema snapshots

| Directory | Status | Contents |
| --- | --- | --- |
| `schema/gaeb-da-xml-3.1-2007-11/` | historical | All XSDs from the three official 2007-11 schema archives. |
| `schema/gaeb-da-xml-3.1-2009-12/` | historical | All XSDs from the two official 2009-12 schema archives. |
| `schema/gaeb-da-xml-3.2-2013-10/` | historical | All downloadable 3.2 package XSDs; the time-contract files carry upstream date 2014-03. |
| `schema/gaeb-da-xml-3.3-2021-05/` | previous | Complete 2021-05 package set, including the package marked beta upstream. |
| `schema/gaeb-da-xml-3.3-2023-01/` | current | Complete current set: the 2023-01 X31 schema plus unchanged 2021-05 packages, as specified by GAEB. |
| `schema/gaeb-da-xml-3.4-2026-03-beta/` | **beta** | All XSDs from the official 3.4 beta archive; not a final GAEB release. |

Same-named support schemas are retained per snapshot so every release is
self-contained. Do not edit the XSDs to make a parser accept them: preserving
the official bytes is the purpose of this corpus.

## Examples

`examples/` intentionally contains a small, phase-diverse corpus rather than
all duplicate upstream samples:

- GAEB DA XML 3.1: `X81`, `X83`, `X84`, and `X86`;
- GAEB DA XML 3.2 time contracts: `X83Z`, `X84Z`, `X86ZE`, and `X86ZR`.

The example bundle linked under 3.1 `2009-12` repeats the older files
byte-for-byte. Their embedded `VersDate` is `2007-06`; they validate against the
2007 schema and not the 2009-12 enumeration, so they live under
`examples/gaeb-da-xml-3.1-2007-11/`. GAEB currently publishes no example for
the current 3.3 release on the download page.

## Provenance and integrity

`SOURCE-MANIFEST.json` records:

- all 23 official GAEB-hosted ZIP URLs linked on the source page;
- each archive's byte size and SHA-256 digest;
- SHA-256 digests for all extracted files.

Verification performed after extraction:

- 23/23 ZIP archives passed CRC checks;
- 126/126 XSD files and 8/8 retained examples are well-formed XML;
- all 125 local `schemaLocation` references resolve inside their snapshot;
- all 8 retained examples validate against their matching official XSD.

A libxml2/lxml XSD 1.0 compilation probe accepts 114/126 individual schemas.
The 12 rejected files are the unmodified `50.1`, `50.2`, `51.1`, and `51.2`
`xs:redefine` variants in both 3.3 snapshots and the 3.4 beta. This is an
upstream schema/redefinition compatibility issue, not missing files: all local
schema references resolve and the archive bytes are preserved.

Retrieved: `2026-08-25T09:44:21Z`.

[source]: https://www.gaeb.de/de/service/downloads/gaeb-datenaustausch/
