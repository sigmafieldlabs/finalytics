---
description: Screen a universe by computed measures and supplied fields, then rank and limit the matches on a chosen date.
---

# Screen a universe

A `Universe` keeps each security on its own dates, with optional volume, dividends and custom fields. `screen` computes measures, applies every rule, sorts matches and returns both a report and typed results.

## Combine measures and fields

The bundled universe includes sectors, valuations, market capitalisations, dividends and a benchmark. This screen combines a text field with an income measure.

[income.py](../../examples/screen-a-universe/income.py)

The benchmark supplies beta, correlation and relative returns but is not a candidate unless requested. The annual risk-free rate supplies Sharpe and Sortino calculations.

## Screen valuation fields

Numeric fields support comparisons and inclusive ranges. Missing values fail a rule rather than silently passing it.

[value-fields.py](../../examples/screen-a-universe/value-fields.py)

The report shows how each rule narrows the universe, the ranked matches, near misses and the distribution around numeric thresholds. The limit cuts the displayed matches after sorting, not before.

## Measure on another date

`as_of` ignores later closes. Include the benchmark when it should be ranked as a candidate as well as used for relative measures.

[date-and-benchmark.py](../../examples/screen-a-universe/date-and-benchmark.py)

A security whose latest permitted close is too old is stale: it remains visible in coverage but does not match. `max_carry_days` controls that tolerance.

## Request only useful measures

Computing every available measure is convenient for exploration. For repeated or narrow screens, name the measures needed; rule and sorting measures are always included automatically.

[measures-and-ranking.py](../../examples/screen-a-universe/measures-and-ranking.py)

Python results contain one record per security. JavaScript returns column arrays, using `NaN` for a missing numeric value. See [Screening](../../methods/screening.md) for every measure and window definition.
