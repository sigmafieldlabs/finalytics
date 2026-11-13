---
description: Compare portfolio and benchmark returns with excess return, tracking statistics, capture ratios and blended benchmarks.
---

# Compare with a benchmark

A benchmark can be an index-level series, a holding or a weighted blend of holdings. Use the same dates and currency basis as the portfolio.

## Arithmetic and geometric excess

Arithmetic excess is portfolio return minus benchmark return. Geometric excess compares their growth factors and is useful when linking relative performance.

[excess.py](../../examples/compare-with-a-benchmark/excess.py)

## Relative statistics

Tracking error is the annualised standard deviation of active returns. The information ratio compares annualised active return with that consistency measure.

[relative-statistics.py](../../examples/compare-with-a-benchmark/relative-statistics.py)

## Capture ratios and beta

Upside and downside capture compound portfolio and benchmark returns over rising and falling benchmark periods. Beta measures sensitivity over all observations.

[capture.py](../../examples/compare-with-a-benchmark/capture.py)

## Use a portfolio's benchmark column

A benchmark price column need not be held. The sample portfolio includes `BENCH`, which its performance can use directly.

[portfolio-benchmark.py](../../examples/compare-with-a-benchmark/portfolio-benchmark.py)

## Blend several holdings

A weighted benchmark is rebalanced monthly in portfolio reports. Weights must be complete and sum to one; the report prints and shows the comparison.

[blended.py](../../examples/compare-with-a-benchmark/blended.py)
