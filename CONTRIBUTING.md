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
cargo clippy --workspace --lib --bins --tests --benches -- -D warnings
cargo clippy --workspace --examples -- -D warnings -A clippy::print_stdout
cargo test --workspace --exclude finalytics-py

# WebAssembly
cargo build --workspace --exclude finalytics-py --target wasm32-unknown-unknown
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p finalytics-wasm --target wasm32-unknown-unknown

# Python
cd bindings/python && maturin build --release -o dist && pip install dist/*.whl && pytest
```

## Reference values

Validation tests compare against independent implementations, such as empyrical, SciPy,
scikit-learn, CVXPY and PyPortfolioOpt. The scripts in `scripts/reference/` generate the
reference values, which are committed as fixtures under each crate's `tests/fixtures/`:

```sh
pip install -r scripts/reference/requirements.txt
python scripts/reference/risk.py
python scripts/reference/optimisation.py
```

Regenerate a fixture only when you add a case or a reference library changes, and review the
difference in the pull request.

## Benchmarks and performance budgets

The benchmarks in `tools/bench` measure the main operations at the sizes of the budgets in
[docs/performance.md](docs/performance.md):

```sh
cargo bench -p finalytics-bench
cargo run --release -p finalytics-bench --bin budgets -- --write-document large.json
bindings/wasm/build.sh && node bindings/wasm/test/budget.mjs large.json
```

For a change that may affect performance, run the benchmarks before and after it on the same
machine and include the comparison in the pull request. Criterion keeps the previous run under
`target/criterion` and reports the change. Update the tables in `docs/performance.md` when a
change moves the numbers materially.

## Fuzzing

Every reader (CSV and JSON) and the numerical entry points have fuzz targets in `fuzz/`. The
checks they run live in `tools/fuzz-checks`, and `cargo test` replays the seed corpus in
`fuzz/corpus` on stable Rust. Fuzzing itself needs a nightly toolchain and
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz):

```sh
cargo install cargo-fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run -O -a document -- -max_total_time=300
```

With `-a` (debug assertions), integer overflow fails the run too. A target fails when the engine
panics, overflows the stack, runs out of memory or time, or when a check fails, for example a
document that does not read back equal to itself. To fix a failure, add the input from
`fuzz/artifacts/<target>/` to `fuzz/corpus/<target>/` with a descriptive name, so that
`cargo test` keeps checking it, and then fix the cause. CI fuzzes each target for a minute on
pull requests that change the engine, and for longer every week.

## Pull requests

- Keep each pull request focused on one change, with tests.
- Write commit messages in the imperative mood ("Add Modified Dietz returns"), with a body that
  explains why when the reason is not obvious.
- Update `CHANGELOG.md` under "Unreleased" for any user-facing change.
- New models need a validation test against an independent reference and a methods page in
  `docs/methods/`, whose Python examples are run by `bindings/python/tests/test_docs.py`.
- Examples in the documentation of the library crates use the paths of the `finalytics` crate,
  as users write them, since docs.rs shows them under `finalytics`. A hidden first line maps
  those paths to the crate being tested; copy it from an example in the same crate.
- The READMEs' code fits 80 columns, and every README program runs, printing the output shown
  after it if there is any. A Rust program in `README.md` is a file in
  `crates/finalytics/examples`, copied as it is, and a shorter snippet is part of one;
  `python3 scripts/check-consistency.py --run-examples` checks both. The Python and JavaScript
  READMEs are run by `bindings/python/tests/test_readme.py` and
  `bindings/wasm/test/readme.test.mjs`. The READMEs' screenshots and the reports they link to
  are built by `scripts/sample-reports.py` from the same programs; if you change a program,
  change that script to match.
- Public APIs follow semantic versioning: breaking changes wait for a major release.

## Licence

By contributing, you agree that your contributions are licensed under the terms of both the MIT
licence and the Apache License, Version 2.0, as described in the [README](README.md#contributing-and-licence).
