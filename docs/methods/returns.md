# Returns

Finalytics measures a portfolio's return three ways: the **time-weighted** return, which removes
the effect of money moving in and out and so measures the investments; the **money-weighted**
return, which measures the investor's experience of those flows; and the **Modified Dietz**
return, an approximation of the time-weighted return from start and end values alone.

All three take end-of-day values $V_t$ on valuation dates and the external flows $F_t$ since the
previous date: deposits and transfers in positive, withdrawals and transfers out negative, each
converted to the base currency at its own date's rate.

## Time-weighted return

Each day's return removes that day's flow, which is assumed to arrive at the **start** of the day
(the default) or at its **end**:

$$r_t = \frac{V_t}{V_{t-1} + F_t} - 1 \quad\text{(start of day)} \qquad r_t = \frac{V_t - F_t}{V_{t-1}} - 1 \quad\text{(end of day)}$$

Daily returns are chain-linked into a growth index $G_t = \prod_{s \le t} (1 + r_s)$, with
$G_0 = 1$, and the return from date $i$ to date $j$ is $G_j / G_i - 1$.

A day with nothing invested (no starting value and no flow) has a return of zero. A day whose
starting value is negative, or that ends with value but started with none, has no defined
return, and is reported as an error naming the date.

## Money-weighted return

The money-weighted return is the internal rate of return $r$ that solves

$$-V_i + \sum_{t} \frac{-F_t}{(1 + r)^{\tau_t}} + \frac{V_j}{(1 + r)^{\tau_j}} = 0$$

where $\tau$ is the time from the start in years of 365 days (actual/365). The starting value
counts as invested on the start date; a flow at the start of a day counts from the previous day.
The rate is found by scanning outwards from zero for a sign change and bisecting, and is
annualised by construction. When no rate between −99.99% and +1,000,000% solves the equation,
the result is None.

## Modified Dietz

$$R = \frac{V_j - V_i - \sum_t F_t}{V_i + \sum_t w_t F_t} \qquad w_t = \frac{D - d_t}{D}$$

where $D$ is the number of days in the period and $d_t$ the days from its start to the flow (one
day fewer for flows at the start of the day, which are invested on the flow date).

## Periods and annualisation

| Period | Starts at the valuation on or before |
| --- | --- |
| `MTD` | The last day of the previous month |
| `QTD` | The last day of the previous calendar quarter |
| `YTD` | The last day of the previous year |
| `nY` | The same day $n$ years earlier (the month end when that day does not exist) |
| `ITD` | The first valuation (since inception) |

A period's cumulative return is $G_{\text{end}} / G_{\text{start}} - 1$. When the period spans at
least 365 days it is also annualised over its actual length:

$$R_{\text{annual}} = (1 + R)^{365.25 / \text{days}} - 1$$

When the history does not reach back to the start of a period, the period has no return (None),
rather than a return over a shorter span.

### Excess return

Against a benchmark given as levels (an index or prices), looked up on the period's start and end
dates and carried forward for up to five days:

$$\text{arithmetic} = R_p - R_b \qquad \text{geometric} = \frac{1 + R_p}{1 + R_b} - 1$$

## Example

```python
>>> import finalytics as fl
>>> performance = fl.Performance.from_values(
...     ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"],
...     [1000.0, 1050.0, 1600.0, 1580.0],
...     [0.0, 0.0, 500.0, 0.0],     # 500 deposited during February
... )
>>> [round(float(r), 6) for r in performance.returns]
[0.05, 0.032258, -0.0125]
>>> round(performance.time_weighted(), 6)
0.070323
>>> round(performance.modified_dietz(), 6)
0.067606
>>> round(performance.money_weighted(), 6)      # annualised
0.304911

```

February's return is $1600 / (1050 + 500) - 1$. The money-weighted return is higher than the
time-weighted one because the deposit arrived before a good month, and it is an annual rate over
a quarter. Over three years, which include a leap day, annualisation uses 1,096 days:

```python
>>> long = fl.Performance.from_values(["2023-03-31", "2026-03-31"], [100.0, 133.1])
>>> found = long.period("3Y")
>>> round(found.cumulative, 6), round(found.annualised, 6)
(0.331, 0.099976)
>>> excess = performance.excess_return([100.0, 104.0], "YTD", dates=["2025-12-31", "2026-03-31"])
>>> round(excess.arithmetic, 6), round(excess.geometric, 6)
(0.030323, 0.029156)

```

## Validation

Worked examples in `crates/finalytics-perf/tests/returns.rs` check each formula, both flow
timings, the treatment of empty and negative starting values, every period boundary (including
month ends and leap years), and excess returns; the money-weighted return is checked by
evaluating the net present value at the rate found. Performance measured from the books is
checked against the same formulas applied to the values by hand.

## References

- CFA Institute, *Global Investment Performance Standards (GIPS) for Firms*, 2020, on
  time-weighted returns and external cash flows.
- Bacon, C. R., *Practical Portfolio Performance Measurement and Attribution*, 3rd edition,
  Wiley, 2023.
- Dietz, P. O., *Pension Funds: Measuring Investment Performance*, Free Press, 1966.
