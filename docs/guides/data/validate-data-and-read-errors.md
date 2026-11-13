---
description: Validate records before reporting, collect problems across files, and distinguish malformed fields from transactions the books cannot apply.
---

# Validate data and read errors

Finalytics reports independent input problems together. Each issue has a severity, a path to its file, line and column where applicable, and a readable message.

## Check well-formed records

`PortfolioData.validate()` returns structural issues without applying the transactions. An empty list means the records themselves are valid.

[valid.py](../../examples/validate-data-and-read-errors/valid.py)

## Collect problems across files

A malformed currency and quantity are returned in one `ValidationError`, rather than stopping after the first bad field.

[several-files.py](../../examples/validate-data-and-read-errors/several-files.py)

## Read the issue fields

The path identifies exactly what to fix. Catch `ValidationError` in Python and JavaScript; match `Error::Validation` in Rust.

[issue-fields.py](../../examples/validate-data-and-read-errors/issue-fields.py)

## Structural validity is not enough

The CSV can be well formed while a sale exceeds the quantity held. Constructing `Portfolio` applies the transactions and reports that books-level error.

[books-error.py](../../examples/validate-data-and-read-errors/books-error.py)

## Handle option errors the same way

Front-door options use the same issue shape. This makes it safe to display all errors beside a form or log them with their paths.

[handle-errors.py](../../examples/validate-data-and-read-errors/handle-errors.py)

> [!TIP]
> Preserve broker transaction identifiers in your CSV. They make error paths actionable and let specific-lot sales name the purchases they close.
