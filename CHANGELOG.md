# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Finalytics 1.0 is a redesign of the library. See the migration guide (to follow) for moving from
0.9.

### Added

- A workspace of crates behind the `finalytics` facade: core, ledger, formats, performance, risk
  and optimisation.
- Python bindings (PyO3, abi3 wheels for Python 3.10 and later).
- WebAssembly bindings and an npm package with TypeScript types.

### Removed

- Data fetching from third-party websites, screeners and technical indicators.
- Charts and HTML reports; the engine now returns data for callers to present.
- Go bindings and the C interface.

## [0.9.0] - 2026-05-01

The last release of the 0.x series. See the `0.9.x` branch.
