#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
if ! command -v cargo >/dev/null 2>&1; then
  echo 'Cargo is not installed in this environment.' >&2
  echo 'Use your configured Fedora Rust development environment, then run this script again.' >&2
  exit 127
fi
cargo test --offline -p nesticles
cargo build --release --locked -p nesticles
exec ./target/release/nesticles
