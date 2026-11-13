---
description: Split portfolio return among holdings, expenses and classifications, and separate local performance from currency effects.
learn: portfolio/2.2
---

# Contributions

Contribution answers “what added to this portfolio's return?” without requiring a benchmark. Daily gains are divided by the value invested and linked by portfolio growth.

## Contributions by holding

Instruments and currency cash balances each have a contribution. Before account-level expenses, holding contributions sum to the time-weighted return.

[holdings.py](../../examples/contributions/holdings.py)

## Include expenses

Advisory fees and account taxes are a separate negative line. Trade fees remain in the holding's own transaction economics rather than being counted twice.

[expenses.py](../../examples/contributions/expenses.py)

## Group by classification

`group_contributions` aggregates holdings by any function, including instrument classifications and a dedicated cash group. The JavaScript binding does not currently export this helper, so this example is available in Python and Rust; JavaScript callers can group the contribution records directly.

[groups.py](../../examples/contributions/groups.py)

## Separate currency effects

A foreign holding has a local return and a base-currency return. Their difference is its currency effect, distinct from Brinson attribution against a benchmark.

[currency.py](../../examples/contributions/currency.py)

## Income, trades and events

Dividends count as income of the paying instrument. Purchases and sales move value between holdings; only external deposits and withdrawals change the portfolio-level flow.

[income.py](../../examples/contributions/income.py)
