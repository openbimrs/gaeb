# `openbim-gaeb` crate instructions

This crate owns GAEB-specific behavior.

- `document` owns byte preservation and supported edits.
- `parser` is streaming and extracts only explicit public views.
- `metadata`, `phase`, and `version` encode GAEB semantics.
- Unknown elements/attributes are never rejected merely because the typed view
  does not expose them.
- Every edit must validate its lexical input, splice only its indexed text
  range, and reparse the complete document before becoming observable.
- Public capability changes require tests and README status updates.
