# OpenBIM.rs GAEB

Canonical repository: <https://github.com/openbimrs/gaeb>
Integration repository: <https://github.com/openbimrs/openbim>

Read `AGENTS.md` before changing the repository and nested `AGENTS.md` files
before editing a crate. Keep both crates independently buildable; the parent
OpenBIM.rs workspace pins this repository as a submodule but is not required for
standalone development.

## Verification

Run `./scripts/gate.sh`. It is the authoritative local and CI gate and decides
success from command exit codes.

## Project conventions

- Rust 2021, MSRV 1.85, AGPL-3.0-or-later.
- Pure Rust and `#![forbid(unsafe_code)]`.
- Use dedicated upstream XML/ZIP libraries rather than a project-owned generic
  codec abstraction. XML mechanics use `quick-xml` directly; BOM/content
  detection, GAEB-specific models, phase semantics, diagnostics, parsing policy,
  and editing stay here.
- Preserve unknown XML and exact input bytes unless a caller explicitly edits a
  supported field.
- Do not claim full XSD validation or full schema coverage without executable
  evidence across the official corpus.
- Do not commit official GAEB schemas/examples until their redistribution terms
  are explicit; use `scripts/fetch-official-references.py` locally.
- Use Keep a Changelog and distinguish implemented capabilities from roadmap.
