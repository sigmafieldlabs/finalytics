---
description: Convert performance series, contribution records, screens and report tables into pandas Series and DataFrames.
---

# Work with pandas

`finalytics.pandas` is a small Python-only conversion layer. It preserves dates, labels and typed records so analysis can continue in pandas without manually unpacking engine results.

## Convert performance series

Returns, growth of one and portfolio value become Series indexed by date. Each has a meaningful name for plotting, joining or saving.

[series.py](../../examples/work-with-pandas/series.py)

The return index excludes the initial valuation date, while growth and value include it. This matches the raw performance object's arrays.

## Convert typed records

`frame` accepts an iterable of Finalytics dataclasses and creates one column per field. Nested records and lists become JSON data rather than opaque string representations.

[contributions.py](../../examples/work-with-pandas/contributions.py)

Use normal pandas selection and aggregation after conversion. Contributions sum to the portfolio's time-weighted return under the library's contribution conventions.

## Convert an entire screen

`securities` accepts either a screen or its raw results. It returns one row per security in universe order, indexed by identifier, with status, match, rank, dates, measures and fields.

[screen-frame.py](../../examples/work-with-pandas/screen-frame.py)

Missing measures become `NaN`, and rank uses pandas' nullable integer dtype. Filtering on `matched` retains the rows kept after sorting and the limit.

## Convert report tables for export

Report tables already carry labels and typed cells. Build a DataFrame from each table's rows and column labels when exporting or joining report data.

[report-frames.py](../../examples/work-with-pandas/report-frames.py)

Use `DataFrame.to_csv`, `to_excel` or another pandas writer as appropriate. Finalytics supplies the consistent tables; pandas owns file output and its optional dependencies.

> [!NOTE]
> Install the pandas extra when it is not already present: `pip install "finalytics[pandas]"`. These helpers do not have JavaScript or Rust equivalents.
