---
description: Measure time-weighted, money-weighted and Modified Dietz returns, control flow timing and select reporting periods.
---

# Measure performance

Time-weighted return removes the effect of external cash flows. Money-weighted return is the investor's annualised internal rate of return, while Modified Dietz approximates the time-weighted result from values and flows.

## Compare the three measures

A deposit before a strong period makes the investor's money-weighted experience differ from the investments' time-weighted return. Modified Dietz offers a simple estimate from the same series.

[three-measures.py](../../examples/measure-performance/three-measures.py)

## Inspect daily returns

Each return is indexed by the date it ends on. A start-of-day flow joins the opening base; an end-of-day flow is removed from the closing value instead.

[daily-returns.py](../../examples/measure-performance/daily-returns.py)

## Choose flow timing

Use the convention that matches your valuation data. The choice affects daily returns, Modified Dietz weights and the dates used in money-weighted cash flows.

[flow-timing.py](../../examples/measure-performance/flow-timing.py)

## Select a reporting period

MTD, QTD, YTD, nY and ITD use the valuation on or before each boundary. If the history does not reach the requested start, the period is absent rather than silently shortened.

[periods.py](../../examples/measure-performance/periods.py)

## Annualise long periods

Cumulative returns of at least one year also have an annualised value based on actual elapsed days. Short or insufficient histories do not acquire a misleading annual figure.

[annualised.py](../../examples/measure-performance/annualised.py)
