#!/usr/bin/env bash
# Runs the checks that CI runs, for Rust, Python and JavaScript.
#
#   scripts/test.sh                  every stage
#   scripts/test.sh rust python      only the stages named: lint, rust, python, js
#
# Every stage runs even when an earlier one fails, and a summary is printed at the end. The exit
# status is non-zero if any stage failed.
#
# The Python stage installs the package into a virtual environment in .venv, created on first
# use with Python 3.11 or later. Set PYTHON to choose the interpreter (default: python3).

set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
root=$(pwd)

if [ $# -eq 0 ]; then
  stages=(lint rust python js)
else
  stages=("$@")
fi

results=()
failed=0

# Prints a heading for each command and runs it; a failure marks the current stage as failed.
run() {
  printf '\n\033[1m$ %s\033[0m\n' "$*"
  if ! "$@"; then
    stage_ok=0
    return 1
  fi
}

# Stops the current stage with a message, for a missing tool.
missing() {
  printf '\n\033[31mmissing:\033[0m %s\n' "$1"
  stage_ok=0
}

venv_python() {
  if [ ! -x .venv/bin/python ]; then
    local python=${PYTHON:-python3}
    if ! "$python" -c 'import sys; sys.exit(sys.version_info < (3, 11))' 2>/dev/null; then
      missing "Python 3.11 or later (set PYTHON to its path, or install one, e.g. brew install python@3.13)"
      return 1
    fi
    "$python" -m venv .venv || return 1
  fi
  .venv/bin/python -m pip install --quiet --disable-pip-version-check maturin pytest pandas jsonschema
}

stage_lint() {
  venv_python || return
  run cargo fmt --all --check
  run cargo clippy --workspace --all-targets -- -D warnings
  run .venv/bin/python scripts/check-consistency.py
  run cargo run --quiet -p finalytics-codegen -- --check
  run .venv/bin/python scripts/generate-types.py --check
}

stage_rust() {
  run cargo test --workspace --exclude finalytics-py
  RUSTDOCFLAGS="-D warnings" run cargo doc --workspace --no-deps --exclude finalytics-py
}

stage_python() {
  venv_python || return
  # maturin installs into the environment named by VIRTUAL_ENV.
  (cd bindings/python && VIRTUAL_ENV="$root/.venv" PATH="$root/.venv/bin:$PATH" \
    run maturin develop --release --quiet) || stage_ok=0
  run .venv/bin/python -m pytest --quiet bindings/python/tests
}

stage_js() {
  local wanted
  wanted=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print }' Cargo.lock)
  if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
    missing "the WebAssembly target: rustup target add wasm32-unknown-unknown"
    return
  fi
  if [ "$(wasm-bindgen --version 2>/dev/null)" != "wasm-bindgen $wanted" ]; then
    missing "wasm-bindgen $wanted: cargo install wasm-bindgen-cli --version $wanted --locked"
    return
  fi
  if ! node -e 'process.exit(Number(process.versions.node.split(".")[0]) < 22 ? 1 : 0)' 2>/dev/null; then
    missing "Node 22 or later"
    return
  fi
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    run cargo test -p finalytics-wasm --target wasm32-unknown-unknown
  run bindings/wasm/build.sh
  local size
  size=$(gzip -9 -c bindings/wasm/pkg/finalytics_bg.wasm | wc -c | tr -d ' ')
  echo "The engine is $size bytes gzipped (budget 2 MiB)"
  run test "$size" -le 2097152
  run node --test bindings/wasm/test/*.test.mjs
  run npx --yes -p typescript@5.9.3 tsc -p bindings/wasm/test
}

for stage in "${stages[@]}"; do
  case $stage in
    lint | rust | python | js) ;;
    *)
      echo "unknown stage: $stage (expected lint, rust, python or js)" >&2
      exit 2
      ;;
  esac
done

for stage in "${stages[@]}"; do
  printf '\n\033[1;34m== %s ==\033[0m\n' "$stage"
  stage_ok=1
  start=$(date +%s)
  case $stage in
    lint) stage_lint ;;
    rust) stage_rust ;;
    python) stage_python ;;
    js) stage_js ;;
  esac
  seconds=$(($(date +%s) - start))
  if [ $stage_ok -eq 1 ]; then
    results+=("$(printf '\033[32mpass\033[0m  %-7s %4ss' "$stage" "$seconds")")
  else
    results+=("$(printf '\033[31mFAIL\033[0m  %-7s %4ss' "$stage" "$seconds")")
    failed=1
  fi
done

printf '\n\033[1mSummary\033[0m\n'
printf '  %s\n' "${results[@]}"
exit $failed
