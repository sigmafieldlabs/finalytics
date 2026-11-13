---
description: Save records and market data as a versioned JSON document, round-trip them exactly, and handle unsupported formats.
---

# Save and load portfolios

A portfolio document holds records, market data and any goals in JSON. Its `format` is `finalytics.portfolio`; `format_version` lets future readers reject formats they do not understand.

## Round-trip a complete portfolio

`to_json()` serialises the portfolio and its market data. `Portfolio.from_json()` rebuilds the books and reproduces the valuation.

[round-trip.py](../../examples/save-and-load-portfolios/round-trip.py)

## Inspect the envelope

The document's top-level fields are deliberately small and explicit. Unknown members are refused rather than silently ignored.

[document-fields.py](../../examples/save-and-load-portfolios/document-fields.py)

## Save to disk

Python and Rust can read a path directly or through standard file APIs. JavaScript keeps serialisation separate from Node's file promises, so the same JSON code also works in a browser with uploaded text.

[disk.py](../../examples/save-and-load-portfolios/disk.py)

## Reject an unsupported version

A document from a newer format fails with an issue at `format_version`; it is never interpreted as if it were the current version.

[bad-version.py](../../examples/save-and-load-portfolios/bad-version.py)

## Save records without market data

`PortfolioData` has its own JSON round trip when you want to keep records separate from prices and exchange rates.

[data-only.py](../../examples/save-and-load-portfolios/data-only.py)
