---
description: Scope reports by dates and accounts, compare with blended benchmarks, and configure risk, attribution and scenario sections.
---

# Choose report options

One `ReportOptions` value can serve the full report and each individual part. A part uses the options relevant to it and ignores the others, while invalid inputs are still validated.

## Limit dates and accounts

Set the report date, first valuation date and account identifiers to focus the analysis. Groupings apply wherever the report provides a breakdown.

[dates-and-accounts.py](../../examples/report-options/dates-and-accounts.py)

When some accounts are omitted, transfers with them count as external money in or out. The provenance states which accounts are covered and how those transfers were treated.

## Use a blended benchmark for attribution

A benchmark can be an instrument or monthly rebalanced weights across holdings and cash. With a classification scheme, the report derives benchmark segments and calculates attribution.

[blend-and-attribution.py](../../examples/report-options/blend-and-attribution.py)

The blend's instruments must have prices. Their classification determines the benchmark segment; holdings or indices without a classification are treated as unclassified.

## Configure risk presentation

Set tail-risk confidences, the rolling window, histogram bins and the length of top lists. These options change the analysis windows or presentation, not the underlying ledger.

[risk-settings.py](../../examples/report-options/risk-settings.py)

Covariance, expected returns, annualisation, flow timing, long-term holding periods and tracking-error benchmark weights are available through the same options when needed.

## Add attribution and scenarios together

The bundled options contain benchmark segments and named scenarios. Combining them with a later start date produces a complete report over a focused period.

[attribution-and-scenarios.py](../../examples/report-options/attribution-and-scenarios.py)

A scenario that cannot run is reported with its own problems without discarding the other scenarios. Attribution inputs wholly outside the report period are omitted with a warning.

See [Reports](../../methods/reports.md) for all option semantics and the treatment of benchmark currencies and periods.
