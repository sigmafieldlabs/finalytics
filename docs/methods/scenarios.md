# Scenarios

A scenario asks what a set of market moves would do to a portfolio as it stands: to its value,
holding by holding and group by group, and to its risk. The moves are either shocks given by the
caller (to instruments, groups of instruments, currencies or factors) or the price and
exchange-rate changes of a historical period, replayed on today's holdings.

## The holdings

A scenario starts from the holdings valued at the end of a date (by default the last price date),
added up across accounts: each instrument, and cash in each currency, with its value $V_h$ in the
base currency. The portfolio must be worth more than zero. Each holding gets a return in its own
currency $l_h$ and a change in its currency's value in the base currency $c_h$, and is worth

$$V_h' = V_h (1 + l_h)(1 + c_h)$$

after. Cash has $l_h = 0$, and base-currency cash $c_h = 0$ too. The portfolio's return is
$\sum_h V_h' / \sum_h V_h - 1$.

## Shocks

An instrument's return in its own currency comes from the first of these that reaches it:

1. its own shock;
2. the group shock that matches it: a classification scheme and value, such as
   `asset_class = Equity`;
3. the factor shocks, through its exposures in the factor model.

An instrument that none of them reaches returns zero, and the result lists it as not shocked. An
instrument without its own shock that matches more than one group shock is an error, reported
with the groups, rather than adding or compounding the shocks; give it its own shock to settle
which applies. A group shock that matches no instrument held is reported as a warning.

A currency shock is the change in the currency's value in the base currency, $-0.1$ when it loses
10%. It moves every instrument priced in the currency, compounding with the instrument's own
return, and cash in the currency. The base currency cannot be shocked.

Instrument, group and currency shocks are returns of $-100\%$ or more. Factor shocks can be any
finite numbers, but the returns they imply must be $-100\%$ or more. Unknown instruments and
factors, and anything shocked twice, are reported together with every other problem.

### Factor shocks

With a factor model whose assets are instrument identifiers, factor returns $f$ give each
instrument in the model the return $b_h^\top f$, from its exposures $b_h$. The factors not
shocked return zero, or, with propagation, their expected return given the shocked ones under
the model's factor covariance $F$ (Kupiec, 1998):

$$f_u = F_{us} F_{ss}^{+} f_s$$

where $s$ and $u$ index the shocked and the other factors and $F_{ss}^{+}$ is the pseudo-inverse
(the inverse when the shocked factors' covariance is not singular). This is the conditional mean
of the other factors when the factor returns are jointly normal. The result reports every
factor's return, given and propagated.

## Historical replay

A historical scenario gives each instrument the change in its price from the start of a period to
its end, and each holding the change in its currency's exchange rate to the base currency:

$$l_h = \frac{P_h(t_1)}{P_h(t_0)} - 1, \qquad c_h = \frac{X_h(t_1)}{X_h(t_0)} - 1$$

Prices and rates are looked up as for valuation: carried forward over a few days and reported as
such. A held instrument with no prices for the period (one that did not exist then, say) is an
error unless a proxy is named for it, another instrument whose prices stand in for its own; the
instrument's own currency still applies. A proxy, when given, is always used.

The result's path values the holdings on every date in the period with a price for one of them,
$\sum_h V_h \frac{P_h(t)}{P_h(t_0)} \frac{X_h(t)}{X_h(t_0)}$, with the worst drawdown along it.

## Effect on value

Each holding reports its value before and after, its weight before and after, its local,
currency and total returns, the change, and how its return was set: its own shock, a group, the
factors, its history, a proxy, cash, or nothing.

By any classification scheme, the holdings add up into groups; instruments without the
classification are in the group "unclassified", and cash in "cash". Each group's contribution is
its change over the portfolio's value before, so the contributions add up to the portfolio's
return.

## Effect on risk

With a risk model of the holdings' returns, a covariance matrix or a factor model labelled with
instrument identifiers and, for cash, labels such as "EUR cash" (as holding returns are), the
result reports the risk at the weights before and at the weights after. Every holding must be in
the model, except base-currency cash, which has no risk. Each report has the volatility
$\sigma = \sqrt{w^\top C w}$, in the units of the model, the decomposition of the volatility by
holding and by group (see [risk decomposition](risk-decomposition.md)), and the parametric value
at risk of the portfolio's value at the model's confidence $\alpha$,

$$\text{VaR} = z_\alpha \, \sigma \, V$$

a loss in the base currency over the model's horizon, with $z_\alpha$ the standard normal
quantile.

Crises tend to raise volatilities and correlations together. The risk after can use a stressed
covariance, with every volatility multiplied by $k$ and every correlation moved a share $a$ of
the way toward one:

$$C' = k^2 \left( (1 - a) \, C + a \, \sigma \sigma^\top \right)$$

with $\sigma$ the volatilities. Its variances are $k^2 \sigma_i^2$ and its correlations
$(1 - a) \rho_{ij} + a$. As a mixture of $C$ and the rank-one matrix $\sigma \sigma^\top$, both
positive semidefinite, $C'$ is positive semidefinite whenever $C$ is.

## Example

```python
>>> import finalytics as fl
>>> portfolio = fl.Portfolio.from_csv(
...     instruments=(
...         "id,kind,currency,class.asset_class,class.region\n"
...         "SPY,fund,USD,Equity,North America\n"
...         "SAP,equity,EUR,Equity,Europe\n"
...         "BOND,bond,USD,Fixed income,North America\n"
...     ),
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency,"
...         "sell_amount,sell_currency,buy_amount,buy_currency\n"
...         "t1,main,2026-01-02,deposit,,,,100000,USD,,,,\n"
...         "t2,main,2026-01-02,buy,SPY,100,500,,,,,,\n"
...         "t3,main,2026-01-02,fx_conversion,,,,,,22000,USD,20000,EUR\n"
...         "t4,main,2026-01-02,buy,SAP,100,190,,,,,,\n"
...         "t5,main,2026-01-02,buy,BOND,250,100,,,,,,\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(
...     prices={
...         "SPY": {"2026-01-02": 500, "2026-02-02": 450, "2026-03-02": 470},
...         "SAP": {"2026-01-02": 190, "2026-02-02": 171, "2026-03-02": 180},
...         "BOND": {"2026-01-02": 100, "2026-02-02": 101, "2026-03-02": 100.5},
...     },
...     fx=[("EUR", "USD", {"2026-01-02": "1.10", "2026-02-02": "1.08", "2026-03-02": "1.09"})],
... )
>>> books = portfolio.books(market)
>>> result = books.shock_scenario(
...     groups={("asset_class", "Equity"): -0.20},
...     instruments={"BOND": 0.02},
...     currencies={"EUR": -0.10},
...     group_by=["region"],
... )
>>> round(result.value_before, 2), round(result.value_after, 2), round(result.portfolio_return, 4)
(95835.0, 81334.9, -0.1513)
>>> for h in result.holdings:
...     print(h.holding.id, round(h.total_return, 4), round(h.change, 2), type(h.source).__name__)
BOND 0.02 502.5 InstrumentSource
SAP -0.28 -5493.6 GroupSource
SPY -0.2 -9400.0 GroupSource
EUR -0.1 -109.0 CashSource
USD 0.0 0.0 CashSource
>>> [(g.group, round(g.contribution, 4)) for g in result.groups[0].groups]
[('Europe', -0.0573), ('North America', -0.0928), ('cash', -0.0011)]

```

SAP falls 20% in euros and the euro 10%, so it loses 28% in dollars. Its own shock takes
precedence for the bond, which gains 2%.

A factor model in which the bond has an exposure of $-6$ to rates, which tend to fall when
equities do, passes an equity shock on to the bond:

```python
>>> model = fl.risk.factor_model(
...     [[1.0, 0.0], [1.2, 0.0], [0.0, -6.0]],
...     [[0.04, -0.0016], [-0.0016, 0.0001]],
...     [0.01, 0.02, 0.0001],
...     assets=["SPY", "SAP", "BOND"],
...     factors=["equity", "rates"],
... )
>>> result = books.shock_scenario(factors={"equity": -0.15}, factor_model=model, propagate=True)
>>> result.factor_returns
[('equity', -0.15), ('rates', 0.006)]
>>> [(h.holding.id, round(h.local_return, 4)) for h in result.holdings[:3]]
[('BOND', -0.036), ('SAP', -0.18), ('SPY', -0.15)]

```

With this covariance the expected change in rates is $-0.0016 / 0.04 \times -0.15 = 0.006$.

January's moves replayed on the March holdings:

```python
>>> history = books.historical_scenario("2026-01-02", "2026-02-02")
>>> round(history.portfolio_return, 4), history.path.worst_drawdown.trough
(-0.0705, datetime.date(2026, 2, 2))

```

Risk before and after an equity fall, with an annual covariance, at 99% confidence, and with
volatilities 50% higher and correlations half way to one after:

```python
>>> covariance = fl.risk.matrix(
...     [
...         [0.0400, 0.0300, 0.0000, 0.0000],
...         [0.0300, 0.0625, 0.0000, 0.0050],
...         [0.0000, 0.0000, 0.0036, 0.0000],
...         [0.0000, 0.0050, 0.0000, 0.0081],
...     ],
...     ["SPY", "SAP", "BOND", "EUR cash"],
... )
>>> stressed = books.shock_scenario(
...     groups={("asset_class", "Equity"): -0.20},
...     risk=covariance,
...     confidence=0.99,
...     stress=fl.risk.CorrelationStress(volatility_scale=1.5, correlation_shift=0.5),
... )
>>> round(stressed.risk_before.volatility, 4), round(stressed.risk_after.volatility, 4)
(0.1361, 0.2151)
>>> round(stressed.risk_before.value_at_risk), round(stressed.risk_after.value_at_risk)
(30354, 41296)

```

The portfolio is worth less after the fall, but in the stressed market its value at risk is more
than a third higher. The bond's share of the risk rises from 1% to 8%, as its correlation with
equities moves from zero to one half.

## Validation

`scripts/reference/scenarios.py` replays the transactions of the shared test portfolio in pandas
to find its holdings, and computes independently, with NumPy and SciPy: a scenario with
instrument, group and currency shocks, its effect by asset class and by region, and its risk
before and after with a stressed sample covariance of the holdings' returns; factor shocks with
one and two factors shocked, with and without propagation (by `numpy.linalg.pinv`); and a
historical replay with and without a proxy, with its path and worst drawdown. The engine agrees
with every value to $10^{-12}$ relative, in Rust, Python and JavaScript. The tests also work a
two-currency portfolio by hand, and check that overlapping group shocks, unknown instruments,
shocks below $-100\%$, a shock to the base currency, holdings missing from the risk model and
periods without prices are reported.

## References

- Kupiec, P. H., "Stress testing in a value at risk framework", *Journal of Derivatives* 6(1),
  1998.
- Basel Committee on Banking Supervision, *Principles for sound stress testing practices and
  supervision*, Bank for International Settlements, 2009.
