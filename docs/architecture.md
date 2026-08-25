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
openbim facade  --------+
```

`quick-xml` supplies streaming XML mechanics. GAEB owns UTF-8 BOM handling,
content detection, element interpretation, mutation, and output policy. IFC and
core must never depend on GAEB.

## Lossless representation

A `Document` owns the complete original byte stream and separately stores
extracted metadata/item views plus byte ranges for supported edits.

- Unchanged output is exactly the input bytes.
- Unknown elements and attributes are preserved because output is not regenerated.
- Extraction resolves every element namespace against the GAEB root namespace;
  vendor elements cannot impersonate GAEB fields by reusing local names.
- A quantity edit replaces one complete text or CDATA range only. Entity-backed
  text is replaced as a whole; comments, processing instructions, nested markup,
  multiple `<Qty>` fields, and fragmented content make the value read-only.
- Every edit reparses before commit; failure leaves the prior document unchanged.
- Empty or duplicate item IDs are rejected for mutation instead of choosing silently.

The extracted `Item` is a common view, not a claim that all exchange phases use
one identical schema type.

## Evidence-aware detection

GAEB 3.1 has a legacy shared namespace; 3.2 and newer encode phase and generation
in phase-specific namespaces. Documents also declare `<Version>` and `<DP>`.
The parser retains namespace and declaration evidence separately, chooses a
namespace-first effective value, and emits stable diagnostics when they disagree.
It never silently erases the conflict.

GAEB 3.4 is represented as beta because the official 2026-03 schema bundle is
published as beta. GAEB 3.3 remains the stable default.

## Resource behavior

Parsing is streaming over the source bytes. Memory is the owned source plus the
extracted item/category summaries; there is no second general-purpose XML tree.
Description text is normalized only in the summary view. Raw XML remains exact.

## Security posture

- Pure Rust and `unsafe` forbidden.
- Content sniffing precedes parsing; extensions are not trusted.
- The root must use one of the supported official GAEB namespace shapes, and
  namespaced descendants are interpreted only when bound to that exact root namespace.
- Undeclared prefixes, duplicate attributes, misplaced/repeated XML declarations,
  extra roots, and non-whitespace trailing content are rejected.
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
