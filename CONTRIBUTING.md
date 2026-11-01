# Contributing to Finalytics

Thank you for your interest in improving Finalytics. This guide explains how to propose changes.

## Before you start

- For bug reports and feature requests, open an issue using one of the templates.
- For larger changes (a new model, a new public API), open an issue to discuss the design
  before writing code.
- Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Development setup

You need a recent stable Rust toolchain, Python 3.10 or later, and Node.js 20 or later.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <the version in Cargo.lock>
pip install maturin pytest
```

## Checks

Run these before opening a pull request. CI runs the same checks.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude finalytics-py

# WebAssembly
cargo build --workspace --exclude finalytics-py --target wasm32-unknown-unknown
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p finalytics-wasm --target wasm32-unknown-unknown

# Python
cd bindings/python && maturin build --release -o dist && pip install dist/*.whl && pytest
```

## Reference values

Validation tests compare against independent implementations, such as empyrical, SciPy and
scikit-learn. The scripts in `scripts/reference/` generate the reference values, which are
committed as fixtures under each crate's `tests/fixtures/`:

```sh
pip install -r scripts/reference/requirements.txt
python scripts/reference/risk_statistics.py
```

Regenerate a fixture only when you add a case or a reference library changes, and review the
difference in the pull request.

## Pull requests

- Keep each pull request focused on one change, with tests.
- Write commit messages in the imperative mood ("Add Modified Dietz returns"), with a body that
  explains why when the reason is not obvious.
- Update `CHANGELOG.md` under "Unreleased" for any user-facing change.
- New models need a validation test against an independent reference and a methods page.
- Public APIs follow semantic versioning: breaking changes wait for a major release.

## Licence

By contributing, you agree that your contributions are licensed under the terms of both the MIT
licence and the Apache License, Version 2.0, as described in the [README](README.md#licence).
