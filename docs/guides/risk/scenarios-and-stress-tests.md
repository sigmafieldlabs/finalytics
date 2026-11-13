---
description: Apply instrument, group, currency and factor shocks to a real portfolio, replay history and compare risk before and after a stressed market.
---

# Run scenarios and stress tests

A scenario revalues today's holdings under market moves that you specify or that occurred over a historical period. It reports the effect on every holding and the whole portfolio, without changing the portfolio's records.

## Shock groups of holdings

Give each group a return in its instruments' local currencies. Grouping the result by the same classification shows each group's contribution to the total portfolio return.

[groups.py](../../examples/scenarios-and-stress-tests/groups.py)

An instrument's own shock takes precedence over a group shock. An instrument without its own shock cannot match more than one shocked group; split or override the overlap explicitly. Unshocked holdings remain in the valuation.

## Combine market and currency moves

Currency shocks compound with local returns rather than adding to them. A 20% local equity fall combined with a 10% fall in EUR against USD gives a base-currency return of `(1 - 0.20) × (1 - 0.10) - 1 = -28%` for EUR holdings.

[currency.py](../../examples/scenarios-and-stress-tests/currency.py)

The same currency move applies to EUR cash. USD cash cannot receive a currency shock in this USD-based portfolio.

## Propagate factor shocks

A supplied factor model translates factor returns into instrument returns. With `propagate`, unshocked factors receive their conditional expected returns given the shocked factors and the factor covariance.

[factors.py](../../examples/scenarios-and-stress-tests/factors.py)

This example supplies all five instruments' exposures, factor covariance and specific variances. Factor shocks apply only after instrument and group shocks, which take precedence.

## Replay a historical period

A historical scenario applies each instrument's price change and each currency's exchange-rate change between two dates to the current holdings. The result also values those holdings along the period and finds the path's worst drawdown.

[historical.py](../../examples/scenarios-and-stress-tests/historical.py)

This is a replay on today's holdings, not a backtest of the holdings and trades that actually existed then. Use [Test and plan](../test/test-and-plan.md) for portfolio simulation through time.

## Supply a historical proxy

Use `proxies` when a held instrument has no prices over the chosen period, or when another series is a more useful stress proxy. Here EXSA receives VTI's historical local-price return.

[proxy.py](../../examples/scenarios-and-stress-tests/proxy.py)

A supplied proxy is always used, even when the instrument has its own history. EXSA's own EUR currency still applies: only its local price return comes from VTI.

## Measure risk before and after

Supply a covariance matrix labelled with every holding, including foreign-currency cash. Base-currency cash is excluded because it has no risk. Confidence controls value at risk; correlation stress raises volatilities and moves correlations towards one for the after-scenario risk.

[risk.py](../../examples/scenarios-and-stress-tests/risk.py)

The covariance is expressed per period over its chosen horizon. Value at risk is a loss in the portfolio's base currency, while volatility is in return units. See [Scenarios](../../methods/scenarios.md) for precedence, propagation and stress formulas.
