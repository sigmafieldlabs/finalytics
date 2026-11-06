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
cp bindings/wasm/js/index.js bindings/wasm/js/reports.js bindings/wasm/js/node.js \
  bindings/wasm/js/worker.js bindings/wasm/js/client.js bindings/wasm/js/client.d.ts "$out"/
# The classes are disposable (`using`), whose types are in TypeScript's esnext.disposable library:
# a reference to it lets the types check without adding it to a project's own settings.
{ printf '/// <reference lib="esnext.disposable" />\n'; cat "$out/finalytics.d.ts"; } > "$out/finalytics.d.ts.tmp"
mv "$out/finalytics.d.ts.tmp" "$out/finalytics.d.ts"
echo "Built $out ($(wc -c < "$out/finalytics_bg.wasm") bytes of WebAssembly, $(gzip -9 -c "$out/finalytics_bg.wasm" | wc -c) gzipped)"
