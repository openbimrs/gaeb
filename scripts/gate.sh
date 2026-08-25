#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/gaeb.
set -euo pipefail

cd "$(dirname "$0")/.."

# bbv-dev hosts concurrent Rust agents. Respect an explicit target; otherwise
# allocate a unique cache so simultaneous gates cannot share mutable artifacts.
if [[ -z "${CARGO_TARGET_DIR:-}" && -d /mnt/backup/build-cache ]]; then
  export CARGO_TARGET_DIR="$(mktemp -d /mnt/backup/build-cache/openbim-gaeb-target.XXXXXX)"
  trap 'rm -rf "$CARGO_TARGET_DIR"' EXIT
fi

cargo fmt --all -- --check
cargo build --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
scripts/check-alias-purity.sh
scripts/test-alias-purity.sh
python3 scripts/check-package-contents.py
cargo package --locked -p openbim-gaeb
cargo package --locked -p gaeb
