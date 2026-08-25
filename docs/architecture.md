# Architecture

## Repository role

`openbimrs/gaeb` is the canonical GAEB source repository.
`openbimrs/openbim` pins a verified commit at `packages/gaeb` and provides
cross-standard integration and the optional `openbim` facade feature.

The child repository is independently buildable. Published dependencies use
versions rather than paths outside this repository.

## Dependency direction

```text
quick-xml  <-  openbim-gaeb  <-  gaeb alias
                         ^
openbim facade  ----------+
```

The dedicated upstream `quick-xml` crate provides streaming XML mechanics.
GAEB owns UTF-8 BOM/content detection, strict input policy, the namespace matrix,
schema-positioned interpretation, diagnostics, mutations, and lossless output
rules. It does not introduce a project-owned generic XML/ZIP abstraction. IFC
and core must never depend on GAEB.

## Lossless representation

A `Document` owns the complete original byte stream and separately stores
extracted metadata/item views plus byte ranges for supported edits.

- Unchanged output is exactly the input bytes.
- Unknown elements and attributes are preserved because output is not regenerated.
- Extraction resolves every element namespace against the GAEB root namespace;
  vendor elements cannot impersonate GAEB fields by reusing local names.
- The item summary recognizes only schema-positioned `Itemlist/Item` elements;
  category context comes only from `BoQBody/BoQCtgy`, and descriptions come from
  the direct `Item/Description` subtree rather than subordinate descriptions.
- A quantity edit replaces one complete text or CDATA range only. Entity-backed
  text is replaced as a whole; comments and processing instructions make a
  readable scalar read-only, while nested markup or multiple `<Qty>` elements
  make the scalar itself ambiguous and therefore absent from the summary.
- Every edit reparses before commit; failure leaves the prior document unchanged.
- Empty or duplicate item IDs are rejected for mutation instead of choosing silently.

The extracted `Item` is a common view, not a claim that all exchange phases use
one identical schema type.

## Evidence-aware detection

GAEB 3.1 uses the two official date namespaces (`200407` and `200706`);
3.2 and newer use an exact phase/generation namespace matrix derived from the
pinned official schemas rather than a Cartesian product. Documents also declare
`<Version>` and `<DP>`. Header evidence is read only from direct
`GAEB/GAEBInfo` fields, while phase evidence is limited to the schema-defined
top-level parent set. Repeated declarations receive explicit duplicate
diagnostics. The parser retains namespace and declaration evidence separately,
chooses a namespace-first effective value, and emits stable diagnostics when
they disagree. It never silently erases the conflict.

`GaebVersion::is_beta()` reports generation-level status only. GAEB 3.4 is beta
because the official 2026-03 bundle is beta; GAEB 3.3 is the stable generation,
although its official matrix also includes phase-specific beta schemas such as
X61, X84P, X98, and X99.

## Resource behavior

Parsing is streaming over the source bytes. Memory is the owned source plus the
extracted item/category summaries; there is no second general-purpose XML tree.
Description text is normalized only in the summary view. Raw XML remains exact.

## Security posture

- Pure Rust and `unsafe` forbidden.
- Content sniffing precedes parsing; extensions are not trusted.
- The root must use an exact namespace present in the pinned official schema
  matrix, and namespaced descendants are interpreted only when bound to that
  exact root namespace.
- The complete source must be UTF-8 and contain only XML 1.0 characters;
  undeclared attribute prefixes, unknown entities, duplicate attributes,
  misplaced/repeated XML declarations, extra roots, and non-whitespace trailing
  content are rejected.
- DTD/DOCTYPE documents are rejected; no external entities are loaded.
- Edits accept only XML Schema decimal lexical forms.
- ZIP handling is outside this crate because GAEB DA XML files are bare XML.

## Standards artifacts

Official downloadable schemas and examples are local verification oracles.
Because explicit public redistribution terms were not found, payloads are
ignored by Git and restored through a pinned URL/SHA-256 fetcher. Source code and
synthetic tests do not embed copied schema definitions.

## Delivery order

1. verify and push the standalone GAEB commit;
2. add/update the superproject submodule pin;
3. run the root integration gate at that exact pin;
4. push the superproject commit.

The pin is the compatibility declaration and rollback point.
