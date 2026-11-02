# Reports

The reports gather, for each screen of a portfolio workstation, everything the screen shows and
exports, computed in one call from the portfolio's books and market data. They contain no new
models: their figures come from the methods on the other pages, and this page defines what each
report holds and the few figures the reports derive themselves.

| Report | Holds |
| --- | --- |
| Holdings | Positions by holding and by account, with cost and unrealised gains in local and base currency; open tax lots with their holding period; allocation by holding, currency, instrument kind, account and each classification; concentration; realised gains and income to date |
| Overview | Headline figures; value and growth of 100 against a benchmark; allocation; risk by holding and by group; each holding's expected return, volatility and share of risk; the efficient frontier with the current portfolio |
| Performance | Value, growth and daily returns; returns over MTD, QTD, YTD, one, three and five years and since inception, time- and money-weighted and Modified Dietz, against the benchmark; monthly and calendar-year returns; contribution by holding and by group, with the top contributors and detractors; flows and income by month; the household by account |
| Risk | Statistics, and statistics against the benchmark; the drawdown series and the largest drawdowns; value at risk and expected shortfall by the historical, parametric and Cornish–Fisher methods at each confidence; the return distribution; the holdings' covariance, correlation and risk decomposition, by holding and by group, and ex-ante tracking error; rolling volatility, Sharpe ratio, beta and tracking error |
| Attribution | Brinson–Fachler attribution against a benchmark's segments, by period and linked, with the portfolio's segments measured from its books |
| Scenarios | Named shock and historical scenarios run together, side by side, with the change in each holding's value in each scenario |

Each report is defined by the same options: the report's date (the last price date by default),
the first valuation date (the first transaction's by default), the classification schemes to
break figures down by, a benchmark (any instrument with prices in the market data), the
risk-free rate, the number of valuation periods in a year (252), when flows happen within the
day, the covariance estimator (Ledoit–Wolf), the confidences of tail risk (95% and 99%), the
window of rolling statistics (63 periods), the number of histogram bins (30), the length of top
lists (10), the long-term holding period (365 days), expected returns for the frontier, benchmark
weights for the tracking error, the benchmark segments for attribution and the scenarios. Each
report uses the options that concern it.

## Provenance

Every report says where its numbers come from: which report it is, the engine and its version,
the report's date and first valuation date, the base currency, the options, the warnings (prices
and rates carried forward, and figures that could not be computed), and its source: the number of
transactions, accounts and instruments, and the SHA-256 digest of the portfolio document with its
market data, as the engine writes it (see the portfolio document's format). The same inputs give
the same digest in Rust, Python and JavaScript, so a saved report can be shown to come from the
inputs it claims, and a report computed again from them gives the same numbers.

## Shapes for charts

Series over time carry a name, a unit (money in a currency, a fraction, a ratio, an index level
or a count), their dates and their values. Breakdowns list a whole's parts with their value and
weight, largest first. Grids have labelled rows and columns, for heatmaps of monthly returns or
correlations, with empty cells where there is no value. Histograms have bins of equal width from
the smallest observation to the largest, each with its count, its share, and the share a normal
distribution with the observations' mean and standard deviation puts in it.

## Tables for export

Any report turns into named tables with typed columns (text, date, number, whole number,
fraction, money in a currency, or yes and no) and one row per record, so that one writer can
produce CSV, spreadsheets or PDF from any report, with the same numbers, headings and units in
every language. The engine writes no files.

## Figures the reports derive

**Valuation dates.** Every date with a price in the market data, from the first valuation date
to the report's date, with both included.

**Monthly and calendar-year returns.** A month's return is the time-weighted return from the
last valuation of the month before (or the first valuation) to the last valuation of the month;
a year's likewise, from the last valuation of the year before. A year is complete when the
valuations reach back into the year before and forward into December. The benchmark's are its
price changes over the same dates.

**Concentration.** The shares of the portfolio in its five and ten largest instruments, and the
Herfindahl–Hirschman index $H = \sum_i s_i^2$ of the instruments' shares $s_i$ of the value
invested in instruments, with the effective number of holdings $1/H$: the number of equal
holdings that would be as concentrated.

**Holdings' risk.** The covariance of the current holdings' returns in the base currency (see
[covariance](covariance.md)), from the first valuation date on which every current holding has a
price, so that a recent purchase does not shorten the history of the others more than it must;
annualised by the number of valuation periods in a year.

**Frontier.** The long-only, fully invested efficient frontier of the current holdings (see
[optimisation](optimisation.md)), with the expected returns given in the options or, for
holdings without one, their annualised mean return over the same history; each holding says
which it has. The current portfolio is shown on the same axes.

**Attribution segments.** A segment's weight is its share of the portfolio's value at the start
of the period, and its return links its holdings' daily gains as a holding's own return does
(see [contribution](contribution.md)). A segment the portfolio does not hold takes the
benchmark's return, so it has no selection effect; a segment the benchmark does not have, such
as cash, takes the portfolio's own return, so it has only an allocation effect. The attribution's
portfolio return in each period is $\sum_i w_i r_i$, which leaves out trading within the
period; the report gives the time-weighted return over the same span beside it, and shorter
periods, such as months, bring the two closer.

**Scenarios.** Every scenario runs on the holdings on the report's date with the holdings'
covariance as its risk model, and may stress it. A scenario that cannot be run is reported with
its problems, and the others still run.

Cash balances of zero are left out of the holdings and the allocation.

## Example

```python
>>> import finalytics as fl
>>> data = fl.PortfolioData.from_csv(
...     instruments="id,kind,currency,class.asset_class\nSPY,fund,USD,Equity\nAGG,fund,USD,Bonds\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency\n"
...         "t1,main,2026-01-30,deposit,,,,10000,USD\n"
...         "t2,main,2026-01-30,buy,SPY,12,500,,\n"
...         "t3,main,2026-01-30,buy,AGG,40,100,,\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(
...     prices={
...         "SPY": {"2026-01-30": 500, "2026-02-27": 520, "2026-03-31": 490, "2026-04-30": 530},
...         "AGG": {"2026-01-30": 100, "2026-02-27": 100.8, "2026-03-31": 101.5, "2026-04-30": 101.0},
...         "INDEX": {"2026-01-30": 1000, "2026-02-27": 1030, "2026-03-31": 1000, "2026-04-30": 1050},
...     }
... )
>>> portfolio = fl.Portfolio(data, market)
>>> report = portfolio.performance_report(benchmark="INDEX", group_by=["asset_class"])
>>> report.provenance.report, report.provenance.as_of.isoformat(), len(report.provenance.source.digest)
('performance', '2026-04-30', 64)
>>> [(p.label, round(p.cumulative, 4), round(p.benchmark, 4)) for p in report.periods if p.cumulative is not None]
[('MTD', 0.0463, 0.05), ('QTD', 0.0463, 0.05), ('ITD', 0.04, 0.05)]
>>> [None if v is None else round(v, 4) for v in report.monthly.values[0][:5]]
[None, 0.0272, -0.0323, 0.0463, None]
>>> [(g.group, round(g.contribution, 4)) for g in report.groups[0].groups]
[('Bonds', 0.004), ('Equity', 0.036), ('cash', 0.0)]
>>> holdings = portfolio.holdings_report(group_by=["asset_class"])
>>> [(s.name, round(s.weight, 4)) for s in holdings.breakdowns[-1].slices]
[('Equity', 0.6115), ('Bonds', 0.3885)]
>>> round(holdings.concentration.effective_holdings, 3)
1.905
>>> [t.name for t in fl.reports.tables(report)]
['periods', 'monthly_returns', 'calendar_years', 'contributions', 'value', 'flows']

```

The year-to-date and longer returns are empty because the history does not reach back to the
start of those periods (see [returns](returns.md)). February, March and April compound to the
4% since inception; equities contributed 3.6 points of it.

## Validation

`scripts/reference/reports.py` replays the shared test portfolio's transactions in pandas and
computes independently, with NumPy: the monthly, calendar-year and period returns, and the
benchmark's; flows and income by month; the concentration of the holdings; the histogram of the
daily returns (`numpy.histogram`); and the attribution segments' weights and returns from the
holdings' daily gains, with their Brinson–Fachler effects. The engine agrees to $10^{-12}$
relative, and the histogram's counts exactly. The tests also check that every figure a report
takes from another method equals that method's result, that the parts of every breakdown, the
contributions and the risk components add up to their wholes, that every table is rectangular,
and that the digest of the shared portfolio is the same in Rust, Python and JavaScript.
