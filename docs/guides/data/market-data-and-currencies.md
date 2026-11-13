---
description: Supply prices and exchange rates, use inverse and pivot conversions, and understand carried and missing observations.
---

# Market data and currencies

Finalytics fetches nothing: you provide end-of-day prices and FX rates. A pair `EUR/USD` is the number of dollars per euro. Prices stay in each instrument's own currency; valuation converts them into the portfolio's base currency.

## Direct rates

Give each pair once in either wide or long CSV form. The direct quote is used before any inferred route.

[direct.py](../../examples/market-data-and-currencies/direct.py)

## Inverse rates

If only `USD/EUR` is present, a request for `EUR/USD` uses its reciprocal. This works for both observed and carried rates.

[inverse.py](../../examples/market-data-and-currencies/inverse.py)

## Cross through a pivot

When neither a direct nor inverse pair exists, Finalytics crosses through USD by default. Set another pivot on `MarketData` when your data uses one.

[pivot.py](../../examples/market-data-and-currencies/pivot.py)

## Carried and stale observations

A missing weekend price is carried from the latest earlier observation for up to five calendar days by default. The lookup exposes its value, observation date and carried flag; beyond the limit it is missing.

[carried.py](../../examples/market-data-and-currencies/carried.py)

> [!WARNING]
> A valuation requiring a stale price or rate fails and lists the missing inputs. Treat carried warnings as a signal to check your market-data feed, not as fresh observations.

## Value a foreign position

The position keeps both local and base values. Its base value includes the FX rate on the valuation date, so currency movement is distinct from local price movement.

[valuation.py](../../examples/market-data-and-currencies/valuation.py)
