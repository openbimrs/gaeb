# Repository instructions

This is the canonical standalone repository for OpenBIM.rs GAEB support.
`openbimrs/openbim` pins it at `packages/gaeb`.

## Directories

- `openbim-gaeb/` — canonical implementation crate.
- `gaeb/` — short-name compatibility crate; pure re-export only.
- `docs/` — architecture and format support documentation.
- `references/` — provenance plus locally downloaded official corpus.
- `scripts/` — authoritative gate and corpus-fetch tooling.

## Invariants

1. The canonical crate builds from crates.io dependencies outside the parent.
2. The `gaeb` alias defines no independent types or behavior.
3. Unedited documents write byte-for-byte identically, including unknown XML.
4. Edits are narrowly spliced and followed by a complete reparse.
5. Detection uses content, namespace, `<Version>`, and `<DP>` evidence; evidence
   disagreements are diagnostics, never silently resolved guesses.
6. Official reference bytes remain untracked while redistribution is unclear.
