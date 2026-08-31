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

## Licensing contributions

Unless an explicitly signed agreement says otherwise, every contribution
submitted to this repository is licensed under `AGPL-3.0-or-later`. Submit only
work that you have the right to license. Identify third-party material and
preserve its license, attribution, and provenance.
