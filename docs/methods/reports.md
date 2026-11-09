# Reports

The reports gather, for each screen of a portfolio workstation, everything the screen shows and
exports, computed in one call from the portfolio's books and market data. They contain no new
models: their figures come from the methods on the other pages, and this page defines what each
report holds and the few figures the reports derive themselves.

| Report | Holds |
| --- | --- |
| Holdings | Positions by holding and by account, with cost and unrealised gains in local and base currency; open tax lots with their holding period; allocation by holding, currency, instrument kind, account and each classification; concentration; realised gains and income to date and within the report's period |
| Overview | Headline figures; value and growth of 100 against a benchmark; allocation; risk by holding and by group; each holding's expected return, volatility and share of risk; the efficient frontier with the current portfolio |
| Performance | Value, growth and daily returns; returns over MTD, QTD, YTD, one, three and five years and since inception, time- and money-weighted and Modified Dietz, against the benchmark; monthly and calendar-year returns; contribution by holding and by group, with the top contributors and detractors; flows and income by month; the household by account |
| Risk | Statistics, and statistics against the benchmark; the drawdown series and the largest drawdowns; value at risk and expected shortfall by the historical, parametric and Cornish–Fisher methods at each confidence; the return distribution; the holdings' covariance, correlation and risk decomposition, by holding and by group, and ex-ante tracking error; rolling volatility, Sharpe ratio, beta and tracking error |
| Attribution | Brinson–Fachler attribution against a benchmark's segments, by period and linked, with the portfolio's segments measured from its books |
| Scenarios | Named shock and historical scenarios run together, side by side, with the change in each holding's value in each scenario |

`Portfolio.report` computes all six from one set of options; the attribution report needs the
benchmark's segments, or a blended benchmark to derive them from, and the scenarios report needs
scenarios, and without them they are left out.

Each report is defined by the same options: the report's date (the last price date by default),
the first valuation date (the first transaction's by default), the classification schemes to
break figures down by, a benchmark (any instrument with prices in the market data, or a blend of
them by weight), the
risk-free rate, the number of valuation periods in a year (252), when flows happen within the
day, the covariance estimator (Ledoit–Wolf), the confidences of tail risk (95% and 99%), the
window of rolling statistics (63 periods), the number of histogram bins (30), the length of top
lists (10), the long-term holding period (365 days), expected returns for the frontier, benchmark
weights for the tracking error, the benchmark segments for attribution and the scenarios. Each
report uses the options that concern it.

## Printed and HTML reports

Every report prints as text, 80 columns wide, with tables, bars and sparklines: `print(report)` in
Python, `report.to_string()` in Rust, `reportText(report)` in JavaScript. The full report also
lays out as one HTML page (`report.to_html(name)`, `report.save_html(path, name)` in Python,
`reportHtml(report, name)` in JavaScript), with the portfolio's name as its title if given: inline
styles and SVG charts with hover details, no scripts or external resources, in the reader's light
or dark theme, and one report to a printed page. `theme="light"` or `"dark"` (`to_html`,
`save_html` and `show` in Python; the last argument of `toHtml`, `saveHtml` and `show` in
JavaScript; `report.with_theme(Theme::Dark)` in Rust) keeps the page in one theme whatever the
reader's setting. On a narrow screen, charts keep a width at which their text can be read and
scroll sideways, as wide tables do, with a shadow at the edge that has more to see. Both show
the report's figures as computed, and both are built in the engine, so they are the same in
every language. A table too wide for the text's columns is split into several, each repeating its
first column; a heading too long for a line wraps between its parts, and the columns of a
correlation matrix are numbered as its rows are.

## Provenance

Every report says where its numbers come from: which report it is, the engine and its version,
the report's date and first valuation date, the base currency, the options, the warnings (prices
and rates carried forward, and figures that could not be computed), and its source: the number of
transactions, accounts and instruments, and the SHA-256 digest of the portfolio document with its
market data, as the engine writes it (see the portfolio document's format). The same inputs give
the same digest in Rust, Python and JavaScript, so a saved report can be shown to come from the
inputs it claims, and a report computed again from them gives the same numbers.

The optimisation, backtest and comparison reports carry a digest of their inputs too
(`context.digest`, printed at the foot of each page): the assets' returns and the settings that
turn them into estimates, and everything the verb was given, as the engine reads it, the
method with its parameters and limits, current and benchmark weights, fixed and given weights,
the benchmark, the validation and the money. Different inputs give different digests, so a
cache or a provenance check can key on it. The title is not an input: printing can give a
report another.

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

**Some accounts.** `accounts`, such as `["brokerage"]`, reports on those accounts alone: the
figures are those of the books kept from their transactions, so each account's lots, cash, gains
and income are its own. A transfer between two accounts reported on nets to nothing, as it does
for the whole portfolio; a transfer with an account left out (a withdrawal matched by a deposit
of the same amount on the same date, or units sent matched by units received) is money in or out,
as a transfer with the outside world is. Each report says which accounts it covers and how many
such transfers there are, such as "Brokerage account only (1 of 2 accounts). 3 transfers with
Retirement account count as money in and out." An unknown account is an error.

**Realised gains and income.** Gains realised on disposals and income received, net of tax
withheld, in the base currency at each one's date: to date, and within the report's period
(`period`), from its first valuation date to its date, both included, as the performance report's
monthly flows count them. The period is the whole history unless the report starts later, such
as `start="2026-01-01"` for this year's; the report then shows the period beside the totals, and
the tables of realised gains and income by year give each year's part of it.

**Concentration.** The shares of the portfolio in its five and ten largest instruments, and the
Herfindahl–Hirschman index $H = \sum_i s_i^2$ of the instruments' shares $s_i$ of the value
invested in instruments, with the effective number of holdings $1/H$: the number of equal
holdings that would be as concentrated.

**Holdings' risk.** The covariance of the current holdings' returns in the base currency (see
[covariance](covariance.md)), from the first valuation date on which every current holding has a
price, so that a recent purchase does not shorten the history of the others more than it must;
annualised by the number of valuation periods in a year. Cash in the base currency is riskless:
it is left out of the estimate, so that a shrinkage estimator does not lend it part of the
average variance, and its variance and covariances are zero. Cash in other currencies carries
its exchange rate's risk.

**Frontier.** The long-only efficient frontier of the current holdings other than base-currency
cash, fully invested in them (see [optimisation](optimisation.md)), with the expected returns
given in the options or, for holdings without one, their annualised mean return over the same
history; each holding says which it has. Riskless cash is left out because the frontier's least
risky point would otherwise be all cash. The current portfolio, with its cash, is shown on the
same axes, and so is the current mix with base-currency cash set aside
(`current_without_base_cash`): the other holdings in the shares they have of each other, which
the frontier, fully invested in them, can be compared with. Cash in other currencies counts as a
holding there, as its value moves with the exchange rate. `Portfolio.assets()` makes the same
estimates, so the current weights valued on them give the current point.

**Fees and taxes.** Fees and taxes charged to an account rather than to a holding, such as an
advisory fee, are shown as their own line in contribution (`expenses`), not as a loss on the cash
that paid them; with the holdings' contributions, they add up to the time-weighted return.

**The benchmark.** A benchmark instrument's levels are in its currency, the one the portfolio's
instruments give it, and are converted to the base currency at the market data's rates on each
valuation date, so a report in euros compares with the benchmark in euros. An instrument the
instruments do not list is taken to be in the base currency; list a benchmark there, with its
currency, for it to be converted. When the market data has no rate for the benchmark's currency,
its levels are compared as they are. Each report's header says which of these holds, such as
"Benchmark: BENCH, in USD, converted to EUR at the market data's rates". A blended benchmark is
given as weights by holding, such as `{"VTI": 0.55, "BND": 0.35, "GLD": 0.1}`, instruments or
indices with prices and cash: it holds them at those weights, each in the base currency,
rebalanced to them after the last valuation date of each month.

**Attribution segments.** With a blended benchmark, the attribution report needs only the
classification scheme (`attribution="asset_class"` in Python, `{ scheme: "asset_class" }` in
JavaScript, `.attribution_by("asset_class")` in Rust): it derives the benchmark's segments in
each calendar month of the report, each segment's weight the sum of its holdings' weights and its
return their weighted return over the month, which is how the blend is held. A holding is in the
segment its instrument is classified in, an index the instruments do not list is
"unclassified", and cash is "cash". A segment's weight is its share of the portfolio's value at the start
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
