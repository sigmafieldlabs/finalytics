#!/usr/bin/env bash
# Build the npm package into bindings/wasm/pkg.
#
# Requires the wasm32-unknown-unknown target, wasm-bindgen-cli at the version in Cargo.lock, and
# wasm-opt from Binaryen at the version scripts/install-binaryen.sh installs. Extra arguments for
# `cargo build` can be passed in EXTRA_CARGO_ARGS.
set -euo pipefail
cd "$(dirname "$0")/../.."

binaryen=$(sed -n 's/^version=//p' scripts/install-binaryen.sh)
if ! wasm-opt --version 2>/dev/null | grep -q "(version_$binaryen)"; then
  echo "bindings/wasm/build.sh needs wasm-opt from Binaryen version $binaryen, found:" \
    "$(wasm-opt --version 2>/dev/null || echo none)." >&2
  echo "Install it with scripts/install-binaryen.sh and put its bin directory on PATH." >&2
  exit 1
fi

out=bindings/wasm/pkg
# shellcheck disable=SC2086
cargo build -p finalytics-wasm --target wasm32-unknown-unknown --profile wasm-release ${EXTRA_CARGO_ARGS:-}
rm -rf "$out"
# The functions' names are left out: they are only for debugging, and were a quarter of the
# package's size.
wasm-bindgen --target web --out-dir "$out" --out-name finalytics --remove-name-section \
  target/wasm32-unknown-unknown/wasm-release/finalytics_wasm.wasm
# -O1 gives the smallest file once gzipped, and runs a little faster than without it; the higher
# levels make the file smaller but compress less well.
wasm-opt -O1 "$out/finalytics_bg.wasm" -o "$out/finalytics_bg.wasm"
cp bindings/wasm/package.json bindings/wasm/README.md LICENSE-MIT LICENSE-APACHE "$out"/
cp bindings/wasm/js/index.js bindings/wasm/js/reports.js bindings/wasm/js/node.js \
  bindings/wasm/js/worker.js bindings/wasm/js/client.js bindings/wasm/js/client.d.ts "$out"/
# The classes are disposable (`using`), whose types are in TypeScript's esnext.disposable library:
# a reference to it lets the types check without adding it to a project's own settings.
{ printf '/// <reference lib="esnext.disposable" />\n'; cat "$out/finalytics.d.ts"; } > "$out/finalytics.d.ts.tmp"
mv "$out/finalytics.d.ts.tmp" "$out/finalytics.d.ts"
echo "Built $out ($(wc -c < "$out/finalytics_bg.wasm") bytes of WebAssembly, $(gzip -9 -c "$out/finalytics_bg.wasm" | wc -c) gzipped)"
