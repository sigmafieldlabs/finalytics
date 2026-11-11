#!/usr/bin/env bash
# Runs a wasm-bindgen test module optimised as the npm package is (see build.sh), so that the
# tests check the code that ships: use it as CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER.
#
# The test build's DWARF debug information is dropped as the module is optimised: wasm-opt
# rewrites only its line table and leaves the rest pointing at code that moved, and the test
# runner, which converts it again, can stop on that ("cannot convert to writable dwarf"). The
# package ships without it, and the tests report their failures without it.
set -euo pipefail
module=$1
shift
optimised="${module%.wasm}.optimised.wasm"
trap 'rm -f "$optimised"' EXIT
wasm-opt -O1 --strip-dwarf "$module" -o "$optimised"
wasm-bindgen-test-runner "$optimised" "$@"
