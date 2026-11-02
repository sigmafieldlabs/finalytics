# Contribution to return

Contribution splits a portfolio's time-weighted return among its holdings, so that the parts add
up exactly to the whole. A holding is an instrument, or cash in one currency, added up across
accounts.

## Definitions

For each holding $k$ and day $t$, the books record its end-of-day value $V_{k,t}$, the value
moved into it by trades $T_{k,t}$ (purchases positive, sales negative, including transfers), and
the income it paid out $I_{k,t}$ (dividends and interest, net of tax withheld), all in the base
currency. The holding's gain for the day is

$$g_{k,t} = V_{k,t} - V_{k,t-1} - T_{k,t} + I_{k,t}$$

A dividend is therefore income of the instrument that paid it, and a trade into cash for the cash
holding. Value moved between holdings, such as a purchase paid from cash, adds to one holding's
$T$ and subtracts from the other's, so only external flows change the portfolio's total.

The day's contribution is the gain over the portfolio's starting value for the day,
$B_t = V_{t-1} + F_t$ for flows at the start of the day or $V_{t-1}$ for flows at the end. Days
are linked by the portfolio's growth up to each day:

$$C_k = \sum_{t=i+1}^{j} \frac{g_{k,t}}{B_t} \cdot \frac{G_{t-1}}{G_i}$$

Because $\sum_k g_{k,t} = B_t \, r_t$, the contributions add up to $G_j / G_i - 1$, the
time-weighted return over the period, with no residual.

### A holding's own return

Each holding's own return links its daily returns $g_{k,t} / (V_{k,t-1} + \max(T_{k,t}, 0))$:
purchases count as invested for the day, while value taken out by a sale does not. It is measured
in the base currency and, from the same series in the holding's own currency, locally. The
**currency effect** is the difference, $R_{\text{base}} - R_{\text{local}}$.

A holding's average weight is the mean, over the days of the period, of its share of the
portfolio's end-of-day value.

Contributions add up by any grouping, such as asset class or region, with
`group_contributions`.

## Conventions

- Spin-offs, mergers and symbol changes move value from the old instrument to the new one at the
  new instrument's market value on the day.
- A holding with no value at the start of a day and no purchase that day has no own return for
  the period (None) if it gains or loses anything; its contribution is still measured.
- The currency effect is the difference of two returns, not a separate attribution term; the
  attribution of returns to allocation, selection and currency against a benchmark is on
  [its own page](attribution.md).

## Example

```python
>>> import finalytics as fl
>>> data = fl.PortfolioData.from_csv(
...     instruments="id,kind,currency\nAAPL,equity,USD\nSAP,equity,EUR\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency,"
...         "sell_amount,sell_currency,buy_amount,buy_currency\n"
...         "t1,main,2026-01-02,deposit,,,,10000,USD,,,,\n"
...         "t2,main,2026-01-02,buy,AAPL,30,150,,,,,,\n"
...         "t3,main,2026-01-02,fx_conversion,,,,,,4400,USD,4000,EUR\n"
...         "t4,main,2026-01-02,buy,SAP,20,200,,,,,,\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(
...     prices={
...         "AAPL": {"2026-01-02": 150, "2026-01-05": 153, "2026-01-06": 151.5},
...         "SAP": {"2026-01-02": 200, "2026-01-05": 198, "2026-01-06": 204},
...     },
...     fx=[("EUR", "USD", {"2026-01-02": "1.10", "2026-01-05": "1.12", "2026-01-06": "1.11"})],
... )
>>> performance = fl.Portfolio(data, market).performance()
>>> round(performance.time_weighted(), 6)
0.01738
>>> for c in performance.contributions():
...     if not c.holding.endswith(" cash"):
...         print(c.holding, round(c.contribution, 6), round(c.return_local, 6),
...               round(c.return_base, 6), round(c.currency_effect, 6))
AAPL 0.0045 0.01 0.01 0.0
SAP 0.01288 0.02 0.029273 0.009273

```

SAP rose 2% in euros and the euro rose against the dollar, adding 0.93% for a dollar investor.
The two contributions, with nothing from the uninvested cash, add up to the portfolio's 1.738%.

## Validation

`crates/finalytics-perf/tests/contribution.rs` checks, on a multi-currency portfolio with a
dividend, a sale, a spin-off, a fee and deposits, that:

- the holdings' values add up exactly to the portfolio's on every day;
- the contributions add up to the time-weighted return to within $10^{-12}$, with flows at the
  start and at the end of the day, and so do the contributions by asset class;
- a foreign holding's local return, base return and currency effect match the prices and rates
  computed by hand, and a dividend counts towards the instrument that paid it.

The Python and JavaScript tests check the same identity on the shared scenario.

## References

- Bacon, C. R., *Practical Portfolio Performance Measurement and Attribution*, 3rd edition,
  Wiley, 2023, on contribution analysis and the linking of daily contributions.
- Carino, D., "Combining attribution effects over time", *Journal of Performance Measurement*
  3(4), 1999, for the linking problem that compounding contributions by cumulative growth solves.
