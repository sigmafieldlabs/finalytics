---
description: Find the exact Python, JavaScript, TypeScript and Rust interfaces for this Finalytics release.
---

# Find an API definition

The task guides teach Finalytics through complete programs. These references list the public interfaces behind them, generated from the same package source that produces this release.

## Python

The Python reference covers the front doors, performance, reports, risk, optimisation, screening, datasets and pandas helpers. It includes public methods and the typed option and result dataclasses, with signatures and the package's own docstrings.

[Open the Python API reference](../../reference/python.md)

Use it when you know the object or result name and need its exact arguments, fields or return type. Start with `Assets`, `Portfolio` or `Universe` if you are choosing an entry point.

## JavaScript and TypeScript

The JavaScript reference is generated from the npm package's TypeScript declarations and JSDoc. It retains interfaces, option unions, classes, exported functions and overloads, including the separate worker client.

[Open the JavaScript and TypeScript API reference](../../reference/javascript.md)

The declarations are useful directly in an editor as well: the npm package includes its `.d.ts` files, so TypeScript can check the same interfaces without generated documentation.

## Rust

Rust's crate documentation includes builders, enums, feature flags and trait implementations that belong to the Rust API rather than the cross-language front doors.

[Open the Rust API reference on docs.rs](https://docs.rs/finalytics)

The guides show complete Rust programs and recorded output. docs.rs provides the crate-level and module-level reference for the published release.

## Keep the references in step with the packages

`python3 scripts/reference_docs.py` regenerates both hosted references. `scripts/guides.py` checks them on every guide run and fails if the committed documentation differs from the package AST, docstrings, declarations or JSDoc.

This makes the references release artifacts, not hand-maintained summaries. Their wording and signatures cannot silently drift from the code being documented.
