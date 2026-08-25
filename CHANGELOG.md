# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-08-25

### Changed

- The release gate now verifies both published packages from clean source.
- Alias purity now fails closed over Cargo dependency, feature, target, build,
  and source shape, with 19 mutation probes and exact package allowlists.
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
  duplicate attributes, misplaced/repeated XML declarations, extra root elements,
  and non-whitespace trailing content.

## [0.1.0] - 2026-08-25

### Added

- Pure-Rust GAEB DA XML recognition for versions 3.1 through 3.4 beta.
- Evidence-aware version and exchange-phase diagnostics.
- Lossless document ownership, BoQ item summaries, and atomic quantity edits.
- Official-reference fetcher with pinned hashes and executable mutation probes.
- Standalone OpenBIM.rs GAEB repository scaffold.

[Unreleased]: https://github.com/openbimrs/gaeb/commits/main
[0.1.1]: https://github.com/openbimrs/gaeb/compare/v0.1.0...v0.1.1
[0.1.0]: https://crates.io/crates/openbim-gaeb/0.1.0
