#!/bin/sh
# Build the browser player and leave the wasm-bindgen package where the desktop app embeds it.
set -eu
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
cargo build -p nesticles-player --release --target wasm32-unknown-unknown
mkdir -p crates/nesticles-player/pkg
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  cargo install wasm-bindgen-cli --version 0.2.130 --locked
fi
wasm-bindgen \
  --target no-modules \
  --out-dir crates/nesticles-player/pkg \
  target/wasm32-unknown-unknown/release/nesticles_player.wasm
rm -f crates/nesticles-player/pkg/BUILD_FAILURE.txt
echo "Player package is in crates/nesticles-player/pkg. Rebuild the desktop app so export embeds it."
