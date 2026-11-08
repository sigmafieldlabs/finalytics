#!/usr/bin/env bash
# Runs a wasm-bindgen test module optimised as the npm package is (see build.sh), so that the
# tests check the code that ships: use it as CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER.
set -euo pipefail
module=$1
shift
optimised="${module%.wasm}.optimised.wasm"
wasm-opt -O1 "$module" -o "$optimised"
exec wasm-bindgen-test-runner "$optimised" "$@"
