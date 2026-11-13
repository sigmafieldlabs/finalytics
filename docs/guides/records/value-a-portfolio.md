---
description: Value positions and cash, reconcile exact totals, inspect holdings and weights, and follow portfolio value through time.
---

# Value a portfolio

Valuation prices every open lot and converts positions and cash into the base currency. Local values remain available for inspecting each instrument without FX conversion.

## Latest valuation

By default, valuation uses the latest price date. Pass an ISO date to value the books at an earlier point.

[latest.py](../../examples/value-a-portfolio/latest.py)

## Reconcile the total

Position values plus cash values equal `total_base` in exact decimal arithmetic. This is a useful check when displaying your own dashboard.

[reconcile.py](../../examples/value-a-portfolio/reconcile.py)

## Inspect holdings and lots

Holdings show quantity and cost by account and instrument, with the underlying lots available separately. They are accounting records even when no current price is available.

[holdings.py](../../examples/value-a-portfolio/holdings.py)

## Compute weights

Weights include instruments and currency cash balances. In base currency they sum to one, subject only to floating-point display rounding.

[weights.py](../../examples/value-a-portfolio/weights.py)

## Follow values and flows

The value series aligns end-of-day values with external flows on every portfolio date. It is the input used to measure performance.

[value-series.py](../../examples/value-a-portfolio/value-series.py)

> [!NOTE]
> Missing prices or rates cause valuation errors. Carried observations are allowed within the carry limit and listed as warnings so stale inputs remain visible.
