# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add caller-provided GAEB XSD graph loading and real streaming instance
  validation through the immutable reviewed `xsd-schema` revision
  `53de66ccb075246a67e5986742cdcdb5deb81267`.
- Add machine-readable schema/support manifests with exact generation, version
  date, phase, namespace, root-schema, fixture, and typed-support claims.
- Add the opt-in `openbim-gaeb-bindings` crate with official-fixture-proven typed
  parse/write/reparse support for GAEB 3.1 X81, X83, and X86.
- Add evidence-tiered business validation: three conformance errors and fifteen
  explicitly advisory interoperability warnings.

### Security

- Reject empty/multiple-root XML, malformed qualified names, comments, and
  processing-instruction targets, XML 1.0-forbidden characters, invalid or
  misordered XML declarations (including declarations after leading
  whitespace), trailing non-whitespace, and all DTDs before XSD instance
  validation; fail schema loading when include/import directives are unresolved
  or report composition errors.
- Bound lossless XML parsing to 256 nested elements, cap XSD preflight at 1,024
  attributes per element and retained schema diagnostics at 4,096 with explicit
  truncation reporting, confine caller-provided schema graphs to private snapshots
  of at most 256 documents, 64 levels, and 8 MiB, and reject `NoUPComps`
  declarations above the six GAEB unit-price component fields before traversal.

### Changed

- Disable publication of all three packages while the required `xsd-schema`
  implementation remains an immutable Git dependency; extracted package checks
  continue to patch and compile that exact revision.

### Fixed

- Enforce Namespaces in XML 1.0 reserved-prefix bindings and prefix declaration
  requirements before XSD validation, and remove Unix and Windows host paths plus
  self-stale commit identity from packaged support-manifest provenance.
- Emit `xml:space` in the XML namespace from generated bindings and require every
  claimed official typed output to pass its exact official XSD and preserve
  decimal values in both text nodes and attributes.
- Preserve GAEB decimal fields with an exact decimal type rather than binary64;
  official typed round trips now compare numeric leaf values as well as reparsing,
  while business arithmetic uses bounded arbitrary precision and fails closed when
  its explicit resource budget is exceeded.
- Scope conformance errors and pairwise lints to coherent evidence-backed release
  tuples; include description IDs and breakdown policy in pair signatures.
- Apply item discounts in commercial price arithmetic, require X84 text complements
  to use baseline-designated `MarkLbl` slots, and resolve arbitrarily large
  `NoUPComps` only from the nearest containing BoQ's `BoQInfo`.
- Treat explicit-empty and XML-whitespace-only `Qty` elements as existing but
  not safely editable rather than reporting them as missing.
- Accept only XML 1.0 `S` characters outside the document element.
- Validate decoded namespace names as URI references before interpreting any
  namespaced content.
- Expand parser mutation coverage from 22 to 25 probes for these regressions.
- Package the exact-version alias against the local canonical candidate before
  that canonical version exists in the registry.
- Keep Cargo diagnostics on stderr so package verification can parse metadata
  even while Cargo reports lock contention or index activity.

## [0.1.2] - 2026-08-25

### Fixed

- Made the alias-contract gate derive the release version from package metadata,
  so package-version bumps remain mutation-verified instead of failing CI.

## [0.1.1] - 2026-08-25

### Changed

- Use the dedicated upstream `quick-xml` crate directly and remove the abandoned
  project-owned XML mechanics wrapper dependency.
- The release gate now verifies both package archives and compiles the packaged
  alias against the candidate canonical package rather than a stale registry crate.
- Alias purity now fails closed over Cargo dependency, feature, target, build,
  and source shape, with 22 mutation probes and exact package allowlists.
- CI now pins its runner and action revisions; local fallback targets are unique
  per gate invocation.
- Removed the shared XML sniffing wrapper dependency. GAEB now uses `quick-xml`
  directly and keeps BOM/content detection beside its adapter policy.

### Fixed

- Resolve descendant namespaces before interpreting GAEB fields, preventing
  vendor-local names from being extracted or selected for mutation.
- Accumulate fragmented quantity character data and fail closed when comments,
  processing instructions, nested markup, or repeated fields prevent one safe edit.
- Edit entity-backed and CDATA-only quantities over their complete lexical ranges.
- Reject missing IDs as mutation handles, spoofed namespaces, undeclared prefixes,
  unknown entities, malformed UTF-8, XML 1.0-forbidden characters, duplicate
  lexical or expanded attribute names, reserved/empty namespace bindings,
  misplaced/repeated XML declarations, extra root elements, and non-whitespace
  trailing content.
- Apply XML 1.0 line-ending and attribute-value normalization to semantic views
  without changing preserved source bytes, and resolve normalized namespace values.
- Restrict namespace recognition to the exact pinned official matrix, including
  the GAEB 3.1 `200706` order namespace, rather than accepting synthetic
  phase/version products.
- Scope metadata, product-specific phase declarations, categories, items, scalars,
  and descriptions to their complete GAEB-qualified schema ancestry; nested item
  content cannot populate an active outer item, and repeated metadata evidence
  receives stable duplicate diagnostics.
- Do not expose a fabricated quantity or description when nested markup,
  duplicate fields, or subordinate descriptions make the common view ambiguous.

## [0.1.0] - 2026-08-25

### Added

- Pure-Rust GAEB DA XML recognition for versions 3.1 through 3.4 beta.
- Evidence-aware version and exchange-phase diagnostics.
- Lossless document ownership, BoQ item summaries, and atomic quantity edits.
- Official-reference fetcher with pinned hashes and executable mutation probes.
- Standalone OpenBIM.rs GAEB repository scaffold.

[Unreleased]: https://github.com/openbimrs/gaeb/commits/main
[0.1.2]: https://github.com/openbimrs/gaeb/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/openbimrs/gaeb/compare/v0.1.0...v0.1.1
[0.1.0]: https://crates.io/crates/openbim-gaeb/0.1.0
