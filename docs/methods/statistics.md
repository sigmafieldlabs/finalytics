# Return statistics and drawdowns

Statistics of a series of $n$ returns $r_1, \dots, r_n$, one per period, with $P$ periods a year
(252 for daily returns by default, 52 weekly, 12 monthly). They apply equally to a portfolio's
returns and to returns supplied directly.

## Definitions

Annual rates given as options, the risk-free rate $R_f$ and the target (minimum acceptable)
return $R_\tau$, are converted to rates per period by compounding:
$r_f = (1 + R_f)^{1/P} - 1$, and likewise $\tau$. With $\bar r$ the mean, $s$ the sample standard
deviation (divisor $n - 1$) and $m_k = \frac1n \sum (r_t - \bar r)^k$ the central moments:

| Statistic | Definition |
| --- | --- |
| Total return | $\prod_t (1 + r_t) - 1$ |
| Annualised return | $(1 + \text{total})^{P/n} - 1$ |
| Volatility | $s \sqrt{P}$ |
| Downside deviation | $\sqrt{\frac1n \sum_t \min(r_t - \tau, 0)^2} \; \sqrt{P}$, over all $n$ periods |
| Sharpe ratio | $\dfrac{\bar r - r_f}{s} \sqrt{P}$ |
| Sortino ratio | $\dfrac{(\bar r - \tau)\,P}{\text{downside deviation}}$ |
| Maximum drawdown | The largest fall from a running peak of the growth $\prod (1 + r_t)$, starting from 1 before the first return, as a negative return |
| Calmar ratio | Annualised return over the size of the maximum drawdown |
| Skewness | $m_3 / m_2^{3/2}$ |
| Excess kurtosis | $m_4 / m_2^2 - 3$ |

Ratios that divide by a dispersion are None when the returns do not vary (or, for the Sortino
ratio, never fall below the target; for the Calmar ratio, never fall).

### Against a benchmark

With benchmark returns $b_t$ over the same periods and active returns $a_t = r_t - b_t$:

| Statistic | Definition |
| --- | --- |
| Active return | The annualised return less the benchmark's annualised return |
| Tracking error | $s_a \sqrt{P}$, the volatility of the active returns |
| Information ratio | $\bar a \, P / \text{tracking error}$ |
| Beta | $\operatorname{cov}(r, b) / \operatorname{var}(b)$ |
| Alpha | $\big(1 + \overline{(r - r_f) - \beta (b - r_f)}\big)^P - 1$ |
| Correlation | $\operatorname{cov}(r, b) / (\sigma_r \sigma_b)$ |

### Drawdowns

From a series of levels (values or a growth index) on dates, a drawdown runs from a **peak** to
the lowest point after it, the **trough**, and ends at the **recovery**, the first date at or
above the peak again. Its depth is $\text{trough} / \text{peak} - 1$. A drawdown still open at the
end of the series has no recovery. Every drawdown is listed, in date order, with the calendar
days from peak to trough and from trough to recovery.

### Rolling statistics

Every statistic has a rolling form over windows of $w$ consecutive returns. Entry $k$ covers
returns $k$ to $k + w - 1$, and so belongs to the date of the last of them.

## Conventions

These follow the empyrical library, so that results can be compared directly:

- volatility and the Sharpe ratio use the sample standard deviation; skewness and kurtosis use
  population moments (as `scipy.stats.skew` and `kurtosis` by default);
- downside deviation averages over all periods, not only those below the target;
- alpha compounds the mean per-period alpha; the information ratio uses the arithmetic mean of
  active returns;
- the annualised return uses the number of periods, not calendar days; for returns measured on
  calendar dates, the [period returns](returns.md) annualise by days.

Returns must be finite and no lower than −100%; each that is not is reported by position.

## Example

```python
>>> import finalytics as fl
>>> monthly = [0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012, 0.031, -0.024, 0.015]
>>> stats = fl.risk.statistics(monthly, periods_per_year=12, risk_free_rate=0.03)
>>> round(stats.annualised_return, 6), round(stats.volatility, 6), round(stats.sharpe_ratio, 6)
(0.075441, 0.081339, 0.570512)
>>> round(stats.sortino_ratio, 6), round(stats.max_drawdown, 6), round(stats.skewness, 6)
(1.493357, -0.042, -0.752941)
>>> benchmark = [0.015, -0.010, 0.025, 0.010, -0.030, 0.012, 0.020, -0.005, 0.010, 0.022, -0.018, 0.011]
>>> relative = fl.risk.relative_statistics(monthly, benchmark, periods_per_year=12, risk_free_rate=0.03)
>>> round(relative.beta, 6), round(relative.tracking_error, 6), round(relative.information_ratio, 6)
(1.366415, 0.022684, 0.617186)

```

Drawdowns of the growth of 100 over the same year:

```python
>>> import numpy as np
>>> levels = 100 * np.cumprod([1.0] + [1 + r for r in monthly])
>>> month_ends = ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31", "2026-04-30", "2026-05-29",
...               "2026-06-30", "2026-07-31", "2026-08-31", "2026-09-30", "2026-10-30", "2026-11-30",
...               "2026-12-31"]
>>> for d in fl.risk.drawdowns(levels, month_ends):
...     print(d.peak, d.trough, d.recovery, round(d.depth, 6))
2026-01-30 2026-02-27 2026-03-31 -0.013
2026-04-30 2026-05-29 2026-09-30 -0.042
2026-10-30 2026-11-30 None -0.024

```

## Validation

`scripts/reference/risk.py` computes every statistic above with empyrical (and SciPy for the
moments) on three series: 756 daily returns with fat tails, 60 skewed monthly returns with
non-zero risk-free and target rates, and 300 daily returns with a crash. The Rust tests
(`crates/finalytics-risk/tests/reference.rs`) require agreement to $10^{-11}$, relative, and the
Python and JavaScript tests check the same fixture. Drawdowns are checked against hand-worked series, including an open
drawdown and a series that never falls.

## References

- Sharpe, W. F., "The Sharpe ratio", *Journal of Portfolio Management* 21(1), 1994.
- Sortino, F. A. and van der Meer, R., "Downside risk", *Journal of Portfolio Management* 17(4),
  1991.
- Young, T. W., "Calmar ratio: a smoother tool", *Futures* 20(1), 1991.
- Jensen, M. C., "The performance of mutual funds in the period 1945–1964", *Journal of Finance*
  23(2), 1968.
- Quantopian, *empyrical* (maintained as empyrical-reloaded), the reference implementation.
