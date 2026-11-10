---
description: Choose the weights of a set of assets with any of the 15 methods, under the limits you set, and read what the result holds.
learn: portfolio/1.4
workstation: portfolio/optimise
---

# Optimise a portfolio

`optimise` chooses the weights of a set of assets by one of 15 methods, under the limits you
set, and returns them with a report on what they are expected to deliver and where their risk
comes from.

## The task in one example

The asset-class dataset holds 30 years of month-end levels of six asset-class indices. With
T-bills as cash, these are the weights with the largest Sharpe ratio, at most 40% in any asset
class:

[optimise.py](../../examples/optimise-a-portfolio/optimise.py)

## Read the result

The weights put 40.0% in bonds, at the limit, 38.3% in US equity and 21.7% in gold, and leave
out international equity and real estate. They are expected to return 7.0% a year at a
volatility of 8.3%: a Sharpe ratio of 0.57.

The weights and the risk tell different stories. US equity is 38.3% of the money but 64.4% of the
risk, its share of the portfolio's variance, while bonds are 40.0% of the money and 11.2% of the
risk. The three assets held count as 2.8 assets in effect.

> [!NOTE]
> The line "On the history it was fitted to" scores the weights on the returns they were chosen
> from, so it flatters them. A backtest tests weights on returns they had not seen.
