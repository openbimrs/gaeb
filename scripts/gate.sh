#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/gaeb.
set -euo pipefail

cd "$(dirname "$0")/.."

# bbv-dev hosts concurrent Rust agents. Never share mutable target artifacts.
if [[ -d /mnt/backup/build-cache ]]; then
  export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-gaeb-target
fi

cargo fmt --all -- --check
cargo build --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
scripts/check-alias-purity.sh
cargo package -p openbim-gaeb
cargo package -p gaeb
