# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-08-25

### Fixed

- Made the alias-contract gate derive the release version from package metadata,
  so package-version bumps remain mutation-verified instead of failing CI.

## [0.1.1] - 2026-08-25

### Changed

- Use the dedicated upstream `quick-xml` crate directly and remove the abandoned
  project-owned `openbim-codec-xml` dependency.
- The release gate now verifies both package archives and compiles the packaged
  alias against the candidate canonical package rather than a stale registry crate.
- Alias purity now fails closed over Cargo dependency, feature, target, build,
  and source shape, with 20 mutation probes and exact package allowlists.
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
  and descriptions to their complete GAEB-qualified schema ancestry; repeated
  metadata evidence receives stable duplicate diagnostics.
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
