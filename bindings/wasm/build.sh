#!/usr/bin/env bash
# Build the npm package into bindings/wasm/pkg.
#
# Requires the wasm32-unknown-unknown target and wasm-bindgen-cli at the version in Cargo.lock.
# Extra arguments for `cargo build` can be passed in EXTRA_CARGO_ARGS.
set -euo pipefail
cd "$(dirname "$0")/../.."

out=bindings/wasm/pkg
# shellcheck disable=SC2086
cargo build -p finalytics-wasm --target wasm32-unknown-unknown --profile wasm-release ${EXTRA_CARGO_ARGS:-}
rm -rf "$out"
wasm-bindgen --target web --out-dir "$out" --out-name finalytics \
  target/wasm32-unknown-unknown/wasm-release/finalytics_wasm.wasm
cp bindings/wasm/package.json bindings/wasm/README.md LICENSE-MIT LICENSE-APACHE "$out"/
echo "Built $out ($(wc -c < "$out/finalytics_bg.wasm") bytes of WebAssembly)"
