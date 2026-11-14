---
description: Produce the complete portfolio report or its six parts as text, HTML and typed tables with provenance.
---

# Read and export reports

A portfolio report contains no separate model: it brings together valuation, performance, risk, attribution and scenarios from the same books and market data. Every figure can be inspected in text, HTML or typed tables.

## Build the complete report

The sample report options include grouping, a benchmark, attribution inputs and scenarios. Naming the report gives the page and its sections a consistent title.

[full-report.py](../../examples/reports/full-report.py)

The six parts are overview, holdings, performance, risk, attribution and scenarios. Attribution and scenarios appear only when their required inputs are supplied.

## Request one part at a time

Use the individual report methods when an application needs only one view. They share the same options and provenance as the corresponding part of the full report.

[individual-parts.py](../../examples/reports/individual-parts.py)

This example prints each part. The complete report and task results expose `show()`; individual sections expose text and typed tables.

## Choose a theme and export tables

Reports are self-contained HTML with inline styles and SVG, and no scripts or external resources. Choose light or dark explicitly when a saved page must ignore the reader's system theme.

[themes-and-tables.py](../../examples/reports/themes-and-tables.py)

`reports.tables` in Python, `reportTables` in JavaScript and `.tables()` in Rust expose stable names, headings, typed columns and rows. The engine does not write spreadsheets; pass these tables to your chosen writer.

## Check provenance

Provenance identifies the report, valuation dates, base currency, options, warnings and source digest. The digest covers the portfolio document and market data.

[provenance.py](../../examples/reports/provenance.py)

The same inputs produce the same digest in every language, making it useful for caches and audit records. Optimisation, backtest, comparison and planning reports carry input digests too.
