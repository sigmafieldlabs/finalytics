# Screening

A screen measures many securities, each on its own history, and keeps those that meet rules on
the measures and on fields given with them. `Universe` holds the securities and
`universe.screen(...)` screens them; the screening report gives how many were measured, each rule
with how many securities met it, the matches in order, and the spread of every measure across the
universe, so that a threshold can be set where it bites.

## The universe

Each security has an identifier, its dates in increasing order and a close on each, and, where
known, a volume, a dividend per share on each date (its ex-date, zero on the others) and
**fields**: values the library does not compute, such as a sector, a price-to-earnings ratio or a
market capitalisation, each a number or text. The securities are not aligned: each keeps its own
dates, so a recent listing or a gap in one history leaves the others whole.

- Closes should include dividends and splits, as adjusted closes do, for total returns. A close
  that is not a number (NaN, or `null` in JSON) is no close: its date is left out, with a warning.
  A close of zero or less, or one that is infinite, is an error.
- A volume that is not a number is unknown. A dividend that is not a number is none paid, unless
  every one of a security's dividends is unknown: then the security has no dividends.
- A field is numbers or text across the universe, and may be missing for some securities. Fields
  may not take the name of a measure, or of `id`, `status`, `passed`, `matched`, `rank`,
  `first_date` or `last_date`.
- The **benchmark** is one of the securities, such as an index. The **risk-free rate** is an
  annual rate, zero by default, converted to a rate a day by compounding:
  $r_f = (1 + R_f)^{1/252} - 1$.

In CSV, the long layout has `id`, `date` and `close` columns (`instrument`, `ticker` or `symbol`
for `id`, `price` for `close`), with `volume` and `dividends` columns if known, one row per
security and date, in any order. The wide layout has a `date` column and one column of closes
per security, empty where it has none. Fields come as a file with an `id` column and one column
per field; a column whose every value is a number holds numbers.

## The date measured to

Each security is measured on its closes up to the date measured to, `as_of`: the latest date in
the universe unless given. Closes after it are left out. A security whose last close is more than
`max_carry_days` (5) days before `as_of` is **stale**: it is listed, with its data measures, but
not measured, and it never matches.

## Windows

The returns and risk measures are over trailing calendar windows ending on `as_of`: a week (7
days), one, three and six months, one, three and five years (12, 36 and 60 months, a day that
does not exist moving back to the end of its month), and the year to date, from the last day of
the previous year. A window starting on day $s$ is measured from its **base**, the security's last
close on or before $s$, to its last close $p_k$ on or before `as_of`:

$$R = \frac{p_k}{p_b} - 1.$$

A window needs a close on or before its first day: a security whose history starts later has no
value for it. Technical measures count closes instead: a 200-day average needs 200 closes.

The daily returns of a window are those between consecutive closes from its base,
$r_j = p_j / p_{j-1} - 1$, whatever the days between them; a gap spans one return.

## Measures

Every measure is computed by default; a screen may ask for some only, and the measures its rules
and its sort use are always computed. Those against a benchmark need one, and those of volumes and
of dividends need some security to have them; without them, the default leaves them out and asking
for them is an error.

| Measure | Definition |
| --- | --- |
| `return_1w`, `return_1m`, `return_3m`, `return_6m`, `return_ytd`, `return_1y` | $R$ over the window. |
| `return_3y`, `return_5y` | A year: $(1 + R)^{1/n} - 1$, with $n$ = 3 or 5 years. |
| `volatility`, `volatility_3y` | $\sigma\sqrt{252}$, with $\sigma$ the sample standard deviation (divisor $n - 1$) of the window's daily returns, over one and three years; zero when they do not vary. |
| `downside_deviation` | $\sqrt{252 \cdot \frac{1}{n}\sum_j \min(r_j - r_f, 0)^2}$ over one year. |
| `max_drawdown`, `max_drawdown_3y` | $\min_j (p_j / \max_{i \le j} p_i - 1)$ over the window's closes from its base: zero or negative. |
| `drawdown` | $p_k / \max_i p_i - 1$ over the whole history to `as_of`. |
| `sharpe_ratio`, `sharpe_ratio_3y` | $\frac{\bar r - r_f}{\sigma}\sqrt{252}$ over one and three years; none when the returns do not vary. |
| `sortino_ratio` | $\frac{252(\bar r - r_f)}{\text{downside deviation}}$ over one year; none without a shortfall. |
| `calmar_ratio` | `return_3y` over $-$`max_drawdown_3y`; none without a drawdown. |
| `value_at_risk` | $-q_{0.05}$, the 5% quantile of the year's daily returns, interpolated linearly between order statistics: the loss of the worst day in twenty, as a positive return. |
| `beta`, `correlation` | Of the daily returns between consecutive dates in the year's window on which both the security and the benchmark have a close: $\text{cov}(r, r_m)/\text{var}(r_m)$ and Pearson's correlation. |
| `relative_return_6m`, `relative_return_1y` | $R$ less the benchmark's $R$ over the same window, each from its own base. |
| `price_to_sma_20`, `price_to_sma_50`, `price_to_sma_200` | $p_k$ over the mean of the last 20, 50 or 200 closes. |
| `sma_50_above_200` | 1 when the mean of the last 50 closes is above that of the last 200, else 0. |
| `rsi_14` | Wilder's relative strength index: with gains $g_j = \max(p_j - p_{j-1}, 0)$ and losses $l_j = \max(p_{j-1} - p_j, 0)$, the averages start as the means of the first 14 and then follow $a_j = (13 a_{j-1} + x_j)/14$ to `as_of`; RSI $= 100\,\bar g / (\bar g + \bar l)$. Needs 15 closes; none for a history that never moves. |
| `macd_histogram` | The MACD line, the exponential average of the closes over 12 days less that over 26, less its own exponential average over 9 days, at `as_of`. Each average is $e_j = e_{j-1} + \alpha (x_j - e_{j-1})$ with $\alpha = 2/(n + 1)$, started at the first value. In the units of the price; needs 34 closes. |
| `from_52w_high`, `from_52w_low` | $p_k$ over the highest (lowest) close of the year's window from its base, less one. |
| `volatility_20d` | $\sigma\sqrt{252}$ of the last 20 daily returns; needs 21 closes. |
| `dividend_yield` | The dividends on dates after the year's first day, to `as_of`, over $p_k$. |
| `average_volume_20d`, `volume_ratio`, `dollar_volume_20d` | The mean volume of the last 20 closes, the last volume over that mean, and the mean of close times volume; none if any of the 20 volumes is unknown. |
| `observations`, `last_price`, `stale` | The closes to `as_of`, $p_k$, and 1 for a stale security, else 0. Every security also has its `first_date` and its `last_date` on or before `as_of`. |

## Rules, sorting and the limit

A numeric rule compares a measure or a numeric field with a number, `<`, `<=`, `>` or `>=`, or a
range, `between` `[low, high]` with both ends included. A text rule tests a text field: `in` or
`not_in` a list of values. A security without the value a rule tests fails it, and a stale
security fails every rule. Without rules, every security that is not stale is kept.

The matches are the securities that meet every rule, sorted by a measure or a numeric field,
largest first (`descending`) or smallest first (`ascending`), those without a value last and ties
in the universe's order, then cut to the limit. Without a sort they keep the universe's order.

All the problems with a screen are reported together, each with its path: an unknown measure or
field (`measures[2]`, `rules[0].field`), a measure the universe cannot give, a comparison that
does not suit what it tests, a range whose low end is above its high end, a missing value, a sort
by text, a limit of zero.

## Coverage

A security is **measured** when it has at least two closes to `as_of` and the history every
rule's measure and the sort's measure needs; **too short** when it has fewer closes or less
history (it is measured, and fails the rules it cannot be judged on); and **stale** as above. For
each rule, the results count the securities that are not stale and pass it on its own, those that
lack its value, and those that pass it and every rule before it. For each measure, they count the
securities that are not stale and have no value.

## The results and the report

The results hold one row per security, in the universe's order: its status, whether it passed
every rule and matched, its rank among the matches, its first and last dates, its value of every
measure computed (none where it has none) and its fields. In JavaScript the same results come as
columns, one `Float64Array` per measure with NaN for no value; in Python,
`finalytics.pandas.securities` turns them into a DataFrame.

The screening report gives the coverage; each rule in words with its counts; the matches, in
order, with the measure or field sorted by, those the rules test and a few of the main measures
(the top 50 in text, every match on the page); and the spread of every measure across the
securities that are not stale: the lowest and highest values and the quartiles, interpolated
linearly between values, with the rules' thresholds marked on a box plot. The page shows them in
the reader's light or dark theme; the text keeps to 80 columns.

## Example

```python
>>> import finalytics as fl
>>> universe = fl.Universe(
...     fl.datasets.file("portfolio/prices.csv"),
...     fields=fl.datasets.file("portfolio/instruments.csv"),
...     benchmark="BENCH",
...     risk_free_rate=0.04,
... )
>>> screen = universe.screen(
...     rules=[
...         ("return_1y", ">", 0),
...         ("class.asset_class", "in", ["Equity", "Real assets"]),
...     ],
...     sort=("sharpe_ratio", "descending"),
... )
>>> r = screen.results
>>> r.matches
['EXSA', 'VTI', 'VNQ']
>>> [(rule.passed, rule.lacking, rule.remaining) for rule in r.rules]
[(4, 0, 4), (4, 1, 3)]
>>> row = r.securities[1]
>>> row.id, row.status, row.rank
('EXSA', 'measured', 1)
>>> measures = ["return_1y", "volatility", "sharpe_ratio", "beta"]
>>> [round(row.values[r.measures.index(m)], 4) for m in measures]
[0.124, 0.1297, 0.6671, 1.1623]
>>> row.values[r.measures.index("return_3y")] is None
True

```

The portfolio dataset's five funds and their benchmark have three years of daily closes, two days
short of a three-year window, so no fund has a three-year measure. Four securities rose over the
year: three funds, and the benchmark, which has no asset class, so it lacks the field and fails
the second rule; the bond and gold funds fell. Of the three that remain, the European equity fund
has the highest Sharpe ratio.

## Speed

Each security is measured in one pass over its own closes, and the securities are independent:
with the `parallel` feature they are measured on several threads, with the same results on any
number. The time grows with the number of closes up to `as_of`. [Performance](../performance.md)
gives the budget: 5,000 securities with ten years of daily closes, about 12 million, in under a
second, natively and in WebAssembly.

## Validation

`scripts/reference/screening.py` computes every measure afresh from the definitions above with
pandas and NumPy (`pct_change`, standard deviations with `ddof=1`, `numpy.quantile`,
`ewm(adjust=False)`, `numpy.cov` and `numpy.corrcoef` on the dates both have a close), on a
universe cut from the S&P 500 prices in `tests/sp500`: twenty companies and the total return
index as the benchmark, with histories that start in 2022, 2024, April 2026 and ten days before
the end, one that stops two weeks early, one with two gaps, closes that are not numbers, and
synthetic volumes, dividends and fields. Four cases cover every measure on the latest date, rules
on fields at an earlier date with a limit, rules on technical measures, and income and volume
sorted by a field. Every value, status, rank, rule count and the matches agree to $10^{-12}$; the
Rust, Python and JavaScript tests hold them to $10^{-9}$. Unit tests check short histories, gaps,
stale securities and benchmarks, missing values, fields, and that every problem with the options
is reported together.

## References

- Wilder, J. W. (1978), *New Concepts in Technical Trading Systems*, Trend Research.
- Appel, G. (2005), *Technical Analysis: Power Tools for Active Investors*, FT Prentice Hall.
- Sharpe, W. F. (1966), "Mutual Fund Performance", *Journal of Business* 39(1), 119–138.
- Sortino, F. A. and Price, L. N. (1994), "Performance Measurement in a Downside Risk
  Framework", *Journal of Investing* 3(3), 59–64.
- Young, T. W. (1991), "Calmar Ratio: A Smoother Tool", *Futures* 20(1), 40.
