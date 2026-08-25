# `openbim-gaeb` plan

Status: initial reader/editor implemented
Updated: 2026-08-25

## Completed

- [x] Content-based GAEB recognition.
- [x] Evidence-aware version and phase detection.
- [x] Lossless document ownership and unchanged write.
- [x] Common BoQ item summaries.
- [x] Atomic quantity edit by unique item ID.
- [x] Official example-corpus regression hook.
- [x] Executable mutation probes for critical detection, validation, and preservation gates.

## Next

- [ ] Add phase-specific typed views based on user demand and conformance fixtures.
- [ ] Add richer lossless edits (unit price, description) with byte-local mutation.
- [ ] Evaluate XSD validation as an optional separate crate/feature; do not burden the lean reader.
- [ ] Add benchmarks on large real-world, redistributable fixtures.
