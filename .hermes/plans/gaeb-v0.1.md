# GAEB v0.1 implementation plan

Status: active
Updated: 2026-08-25

## 1. Goal

Ship a useful pure-Rust GAEB DA XML library in `openbimrs/gaeb`, then pin its
verified commit as `openbimrs/openbim/packages/gaeb`.

## 2. Constraints

- Standalone repository, versioned crates.io dependencies, MSRV 1.85.
- GAEB uses `quick-xml` directly and owns BOM/content detection as adapter policy.
- GAEB owns phase/version semantics, BoQ views, diagnostics, and supported edits.
- Unknown XML must survive; unchanged writes must be byte-identical.
- Official XSD/example redistribution terms are not explicit, so payloads remain
  local and reproducibly downloadable rather than committed.
- Shared parent `master`: integrate by verified child commit and narrow parent
  compare-and-swap landing only.

## 3. Current unknowns

- Whether GAEB will grant explicit schema/example redistribution permission.
- How much vendor-specific XML appears outside the official examples.
- Which fields should join quantity as the next lossless editable surface.

## 4. Workstreams

1. Repository scaffold, documentation, CI, reproducible reference fetch.
2. Content/version/phase detection with conflict diagnostics.
3. Streaming extraction of GAEB metadata and BoQ item summaries.
4. Byte-preserving document ownership and narrow quantity editing.
5. Canonical plus short alias crates and OpenBIM facade integration.

## 5. Validation strategy

- TDD with synthetic fixtures for 3.1, 3.2, 3.3, and 3.4 beta.
- Exact-byte unchanged round trips and mutation probes against the assertion.
- Quantity-edit diff constrained to the intended text range, then full reparse.
- Parse all locally downloaded official examples.
- Standalone fmt/build/test/clippy/rustdoc/package gate.
- Clean-clone gate and parent full integration gate at the exact child pin.

## 6. Risk and rollback

- This is not full XSD validation and must not be presented as such.
- Schema evolution is surfaced through raw phase/version values and diagnostics;
  unsupported content remains preserved.
- Rollback is the parent submodule pin; canonical child history remains intact.

## 7. Next action

Write failing public-contract tests before implementing parser modules.
