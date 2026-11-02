# Performance attribution

Attribution explains a portfolio's return relative to its benchmark by the decisions that
produced it: how much of the portfolio was put in each segment (an asset class, a sector, a
region), what was held within each segment, and, for a portfolio in several currencies, the
exposure to each currency. [Contribution](contribution.md), by contrast, splits the portfolio's
own return among its holdings, with no benchmark.

## Brinson–Fachler

In one period, let segment $i$ have portfolio weight $w_i$ and return $r_i$, and benchmark
weight $b_i$ and return $s_i$, with the weights at the start of the period. The portfolio returns
$R = \sum_i w_i r_i$ and the benchmark $B = \sum_i b_i s_i$. Brinson and Fachler (1985) split
the excess return $R - B$ into three effects for each segment:

| Effect | Formula | Positive when the portfolio |
| --- | --- | --- |
| Allocation | $(w_i - b_i)(s_i - B)$ | Overweights segments that beat the whole benchmark |
| Selection | $b_i (r_i - s_i)$ | Holds assets that beat their segment of the benchmark |
| Interaction | $(w_i - b_i)(r_i - s_i)$ | Overweights the segments where its selection beat the benchmark |

Since $\sum_i (w_i - b_i) B = 0$, the effects of all the segments add up to $R - B$ exactly.
Measuring allocation against $s_i - B$ rather than $s_i$ alone (as Brinson, Hood and Beebower,
1986, do) means that overweighting a segment that returns less than the benchmark counts against
the portfolio even when the segment's return is positive.

The weights of each side must add up to one, so that a portfolio's cash is a segment like any
other. Segment returns are fractions above $-1$, and segments are identified by name, which must
be unique within a period.

## Linking periods

Over several periods, effects that add up within each period do not add up to the excess of the
compounded returns, $R - B$ with $R = \prod_t (1 + R_t) - 1$. Linking scales each period's
effects by a coefficient $c_t$ chosen so that $\sum_t c_t (R_t - B_t) = R - B$; every effect,
summed over the periods with these coefficients, then adds up to $R - B$ exactly.

| Method | Coefficient | Reference |
| --- | --- | --- |
| Cariño (default) | $c_t = k_t / k$, with $k_t = \dfrac{\ln(1 + R_t) - \ln(1 + B_t)}{R_t - B_t}$ and $k$ the same for $R$ and $B$ | Cariño (1999) |
| Menchero | $c_t = M + \dfrac{R - B - M \sum_t (R_t - B_t)}{\sum_t (R_t - B_t)^2}(R_t - B_t)$, with $M = \dfrac{(R - B)/n}{(1 + R)^{1/n} - (1 + B)^{1/n}}$ | Menchero (2000) |

Cariño's coefficients come from logarithmic returns; Menchero's are as close to a single constant
$M$ as the constraint allows, so they change the effects of each period as evenly as possible.
One period needs no linking, and its coefficient is one.

In floating point:

- Where the portfolio's and benchmark's returns are equal, or too close to tell apart, $k_t$
  takes its limit $1/(1 + R_t)$ (and $k$ likewise), and $M$ its limit $(1 + R)^{(n - 1)/n}$.
- Menchero's coefficients are computed as
  $\left(M \sum_s g_s (g_s - g_t) + g_t (R - B)\right) / \sum_s g_s^2$, with $g_t = R_t - B_t$:
  the formula in the table rearranged, so that a coefficient much smaller than $M$ does not come
  from cancelling two numbers the size of $M$.
- When what Menchero's correction would distribute, $R - B - M \sum_t g_t$, is within rounding
  error of zero, it is left out, since dividing rounding error by the squares of tiny differences
  would only add noise.

Both methods take logarithms or roots of $1 + R_t$, so linking needs every period's returns above
$-100\%$. With short positions a portfolio can lose more than it is worth; such a period can be
attributed on its own but not linked with others, and is reported as an error. So are periods
whose returns differ so much in size that floating point cannot compute coefficients that
reproduce the excess return.

The linked result reports each segment's effects, the totals of each effect, and each period's
own attribution before linking, with its coefficient. A segment that is missing from some periods
is linked over the periods in which it appears.

## Currency attribution

For a portfolio in several currencies, Karnosky and Singer (1994) measure the market effects on
returns in excess of cash, and the currency effects on the return of cash in each currency, so
that interest-rate differences are attributed to currency rather than to the markets. Let segment
$i$ be in currency $z$, with returns $r_i$ and $s_i$ in that currency, and let the currency have
the spot return $e_z$ in the base currency (zero for the base currency itself) and the cash
return $c_z$, its short-term interest rate over the period. Then:

- **Market effects** are Brinson–Fachler on the excess returns $x_i = r_i - c_z$ and
  $y_i = s_i - c_z$.
- **Currency effects.** Cash in currency $z$ returns $f_z = (1 + c_z)(1 + e_z) - 1$ in the base
  currency. With $u_z$ and $v_z$ the portfolio's and benchmark's exposures to the currency and
  $F = \sum_z v_z f_z$ the benchmark's currency return, the currency allocation effect is
  $(u_z - v_z)(f_z - F)$.
- **Residual.** Without given exposures, $u_z$ (or $v_z$) is the currency's share of the segment
  weights and a side's return is its unhedged return in the base currency, with each segment
  returning $(1 + r_i)(1 + e_z) - 1$. The additive model leaves out the cross terms,
  $\sum_i w_i e_z x_i$ for the portfolio, and the residual is the portfolio's cross terms less the
  benchmark's. With given exposures, as for a hedged portfolio, a side's return is the model's,
  $\sum_i w_i x_i + \sum_z u_z f_z$, which leaves nothing out.

The market, currency and residual terms add up to the excess return in the base currency, and
are linked over periods as above, with the residual linked like the effects. Each period lists
the returns of every currency of its segments and exposures.

## Example

```python
>>> import pandas as pd
>>> import finalytics as fl
>>> segments = pd.DataFrame(
...     {
...         "portfolio_weight": [0.4, 0.3, 0.3],
...         "portfolio_return": [0.20, -0.05, 0.06],
...         "benchmark_weight": [0.4, 0.2, 0.4],
...         "benchmark_return": [0.10, -0.04, 0.08],
...     },
...     index=["UK", "Japan", "US"],
... )
>>> year = fl.performance.attribution_period("2026-01-01", "2026-12-31", segments)
>>> result = fl.performance.brinson_fachler([year])
>>> round(result.portfolio_return, 4), round(result.benchmark_return, 4)
(0.083, 0.064)
>>> for s in result.segments:
...     print(s.name, round(s.allocation, 4), round(s.selection, 4), round(s.interaction, 4))
UK 0.0 0.04 0.0
Japan -0.0104 -0.002 -0.001
US -0.0016 -0.008 0.002
>>> round(result.allocation, 4), round(result.selection, 4), round(result.interaction, 4)
(-0.012, 0.03, 0.001)

```

The portfolio beat its benchmark by 1.9%. Overweighting Japan, which returned less than the
benchmark, and underweighting the US, which returned more, cost 1.2%; the choice of UK holdings
added 4%.

Over two quarters, the excess returns of the periods add up to $-0.1\%$, while the compounded
returns differ by $-0.195\%$. Linking accounts for the difference:

```python
>>> def quarter(start, end, portfolio, benchmark):
...     frame = pd.DataFrame(
...         {
...             "portfolio_weight": portfolio[0],
...             "portfolio_return": portfolio[1],
...             "benchmark_weight": benchmark[0],
...             "benchmark_return": benchmark[1],
...         },
...         index=["equities", "bonds"],
...     )
...     return fl.performance.attribution_period(start, end, frame)
>>> quarters = [
...     quarter("2026-01-01", "2026-04-01", ([0.6, 0.4], [0.08, 0.01]), ([0.5, 0.5], [0.06, 0.015])),
...     quarter("2026-04-01", "2026-07-01", ([0.65, 0.35], [-0.05, 0.02]), ([0.5, 0.5], [-0.04, 0.02])),
... ]
>>> linked = fl.performance.brinson_fachler(quarters)
>>> [round(p.portfolio_return - p.benchmark_return, 4) for p in linked.periods]
[0.0145, -0.0155]
>>> round(linked.excess_return, 6)
-0.001951
>>> round(linked.allocation + linked.selection + linked.interaction, 6)
-0.001951
>>> [round(p.linking_coefficient, 4) for p in linked.periods]
[0.9822, 1.0447]
>>> round(fl.performance.brinson_fachler(quarters, "menchero").allocation, 6)
-0.004985

```

For currencies, a portfolio based in dollars holds US and European equities, in a year when the
euro gains 3% and cash returns 4% in dollars and 2.5% in euros:

```python
>>> import datetime
>>> T = fl.types
>>> def currency_year(portfolio_exposures=None):
...     return T.CurrencyAttributionPeriod(
...         start=datetime.date(2026, 1, 1),
...         end=datetime.date(2026, 12, 31),
...         segments=[
...             T.CurrencySegment(name="US", currency="USD", portfolio_weight=0.5,
...                               portfolio_return=0.03, benchmark_weight=0.6, benchmark_return=0.035),
...             T.CurrencySegment(name="Europe", currency="EUR", portfolio_weight=0.5,
...                               portfolio_return=0.05, benchmark_weight=0.4, benchmark_return=0.04),
...         ],
...         currencies=[
...             T.CurrencyReturn(currency="USD", spot_return=0.0, cash_return=0.04),
...             T.CurrencyReturn(currency="EUR", spot_return=0.03, cash_return=0.025),
...         ],
...         portfolio_exposures=portfolio_exposures,
...     )
>>> result = fl.performance.karnosky_singer([currency_year()])
>>> round(result.excess_return, 6)
0.00627
>>> [round(x, 6) for x in (result.allocation, result.selection, result.interaction)]
[0.002, 0.001, 0.0015]
>>> [(c.currency, round(c.allocation, 6)) for c in result.currencies], round(result.residual, 6)
([('USD', 0.00063), ('EUR', 0.000945)], 0.000195)
>>> hedged = fl.performance.karnosky_singer([currency_year({"USD": 0.75, "EUR": 0.25})])
>>> [(c.currency, round(c.allocation, 6)) for c in hedged.currencies], round(hedged.residual, 6)
([('USD', -0.000945), ('EUR', -0.001417)], -0.00018)

```

Unhedged, the portfolio's 10% overweight in euros added 0.16%, because euro cash returned 5.6% in
dollars against the benchmark's 4.6%. Hedging half its euros into dollars turns that into a cost
of 0.24%; the market effects are unchanged, and the residual is now the benchmark's cross terms
alone.

## Validation

`scripts/reference/attribution.py` implements Brinson–Fachler, both linking methods and
Karnosky–Singer independently in NumPy, from the formulas in the papers, for four quarters of
four segments, and for currency attribution in three currencies with and without given
exposures. The engine's effects for each period, linked effects and residuals agree with it to
$10^{-15}$, and its linking coefficients to $10^{-13}$ relative (they divide by differences of
logarithms, where implementations of $\ln(1 + x)$ differ in the last place). The tests also work
single periods by hand, and check that every effect adds up to the excess return over several
periods with either linking, that rounding error is not amplified into Menchero's coefficients,
and the validation of weights, periods and currencies. The fuzz tests check, for arbitrary
weights and returns, that the effects add up to within rounding of the inputs' size and that the
linked effects are the periods' effects times their coefficients. The Python and JavaScript tests
repeat the reference checks.

## References

- Brinson, G. P. and Fachler, N., "Measuring non-US equity portfolio performance", *Journal of
  Portfolio Management* 11(3), 1985.
- Brinson, G. P., Hood, L. R. and Beebower, G. L., "Determinants of portfolio performance",
  *Financial Analysts Journal* 42(4), 1986.
- Cariño, D., "Combining attribution effects over time", *Journal of Performance Measurement*
  3(4), 1999.
- Menchero, J., "An optimized approach to linking attribution effects over time", *Journal of
  Performance Measurement* 5(1), 2000.
- Karnosky, D. S. and Singer, B. D., *Global Asset Management and Performance Attribution*,
  Research Foundation of the Institute of Chartered Financial Analysts, 1994.
- Bacon, C. R., *Practical Portfolio Performance Measurement and Attribution*, 2nd ed., Wiley,
  2008.
