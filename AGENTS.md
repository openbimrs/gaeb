# Repository instructions

This is the canonical standalone repository for OpenBIM.rs GAEB support.
`openbimrs/openbim` pins it at `packages/gaeb`.

## Directories

- `openbim-gaeb/` — canonical lossless, validation, and support-manifest crate.
- `openbim-gaeb-bindings/` — opt-in generated bindings for fixture-proven profiles only.
- `gaeb/` — short-name compatibility crate; pure re-export only.
- `docs/` — architecture and format support documentation.
- `references/` — provenance plus locally downloaded official corpus.
- `scripts/` — authoritative gate and corpus-fetch tooling.
- `worktrees/` — gitignored scratch area for disposable worktrees and review
  trees (see "Scratch worktrees" below). Never committed.

## Scratch worktrees

Disposable worktrees, immutable-review trees, and fresh-clone gate runs belong
under `worktrees/` inside this checkout — **not** in `~/projects/` (which is
reserved for real concerns per the root `HERMES.md`) and not scattered across
`/mnt/backup/` with `mktemp` suffixes.

```bash
# from the repo root
git worktree add --detach worktrees/review-<short-sha> <commit>
# ... run gates, mutations, packaging checks ...
git worktree remove worktrees/review-<short-sha>
```

Rules:

- Name them `worktrees/<purpose>-<short-sha>` so an abandoned tree is
  self-identifying.
- Point `CARGO_TARGET_DIR` at `/mnt/backup/build-cache/` — build artifacts are
  large and must not live inside the checkout.
- Remove with `git worktree remove` (falls back to `git worktree prune`), so the
  parent repo's worktree registry never accumulates stale entries.
- Before deleting any scratch tree, confirm `git status --porcelain` is empty;
  if not, snapshot it to a `refs/wip/*` ref first rather than discarding it.

## Invariants

1. The canonical crate builds standalone with versioned dependencies. Until the
   required `xsd-schema` fixes are released upstream, its one Git dependency is
   pinned to an immutable reviewed revision; the package gate must patch the
   extracted crate to that same revision. All three package manifests must keep
   `publish = false`; remove the pin and re-enable publication only after an
   equivalent upstream registry release is available.
2. The `gaeb` alias defines no independent types or behavior.
3. Unedited documents write byte-for-byte identically, including unknown XML.
4. Edits are narrowly spliced and followed by a complete reparse.
5. Detection uses content, namespace, `<Version>`, and `<DP>` evidence; evidence
   disagreements are diagnostics, never silently resolved guesses.
6. Official reference bytes remain untracked while redistribution is unclear.
7. `support-matrix.csv` is the only runtime XSD/typed claim source; a typed row is
   non-empty only after official-fixture parse → write → reparse, exact-root XSD
   validation, and exact decimal-value retention.
8. Generated bindings stay in the sibling crate so the lossless core does not pay their compile cost.
