# Contributing

## Setup

```bash
git clone https://github.com/openbimrs/gaeb.git
cd gaeb
./scripts/gate.sh
./scripts/mutation-probes.py
```

Use Rust 1.85 or newer. Keep the workspace independently buildable from
`openbimrs/openbim`.

## Capability changes

- Add a failing test before implementing behavior.
- Preserve unknown XML and exact unchanged round trips.
- Do not infer schema/business validation from successful parsing.
- Run the official corpus test when local references are available.
- Update README capability tables and `CHANGELOG.md` honestly.

## References

Run `./scripts/fetch-official-references.py` to restore checksum-pinned official
schemas/examples locally. Do not force-add ignored standard artifacts without a
confirmed redistribution license and an explicit repository decision.

## Pull requests

Run `./scripts/gate.sh` and `./scripts/mutation-probes.py`. Keep commits focused
and include the commands that prove the changed behavior.
