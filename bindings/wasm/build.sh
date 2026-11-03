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
cp bindings/wasm/js/index.js bindings/wasm/js/datasets.js bindings/wasm/js/node.js bindings/wasm/js/worker.js \
  bindings/wasm/js/client.js bindings/wasm/js/client.d.ts "$out"/
cat bindings/wasm/js/datasets.d.ts >> "$out/finalytics.d.ts"
# The S&P 500 prices are files of the package, read on demand, not part of the WebAssembly.
mkdir -p "$out/data/sp500"
cp crates/finalytics/data/sp500/prices.csv crates/finalytics/data/sp500/index.csv "$out/data/sp500/"
echo "Built $out ($(wc -c < "$out/finalytics_bg.wasm") bytes of WebAssembly, $(gzip -9 -c "$out/finalytics_bg.wasm" | wc -c) gzipped)"
