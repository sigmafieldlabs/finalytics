---
description: Build ordered formula fields and combine moving averages, RSI, MACD, price ranges and volume in a screen.
---

# Use formulas and technicals

Formula fields and technical measures let a screen express a strategy without modifying the universe. Formulas are evaluated as arithmetic, never executed as code.

## Compose formulas in order

Each formula can use measures, numeric fields and formulas defined before it. The resulting fields are available to later formulas, rules and sorting.

[ordered-formulas.py](../../examples/formulas-and-technicals/ordered-formulas.py)

A missing input, division by zero or non-finite result produces no value for that security. It then fails rules on that formula and is counted in the data coverage.

## Combine trend and momentum

Moving-average ratios, the moving-average crossover, Wilder's RSI and MACD describe different aspects of a price history. Use them together to make the intended setup explicit.

[trend-and-momentum.py](../../examples/formulas-and-technicals/trend-and-momentum.py)

Technical windows count closes rather than calendar time. A security without enough closes has no value and fails any rule requiring it.

## Add price ranges and activity

Distance from the annual high or low can be combined with relative volume and dollar volume to avoid ranking inactive securities.

[ranges-and-volume.py](../../examples/formulas-and-technicals/ranges-and-volume.py)

Range distances are returns: distance below the high is negative, while distance above the low is positive. Volume measures need complete data over their calculation window.

## Use the formula functions

Supported functions include absolute value, square root, natural logarithm, minimum and maximum. Parentheses and normal arithmetic precedence apply.

[formula-functions.py](../../examples/formulas-and-technicals/formula-functions.py)

> [!NOTE]
> Formula errors are reported at the formula's own path, with unknown names and ordering problems included alongside the screen's other validation issues.

See [Screening](../../methods/screening.md) for the formula grammar and exact technical definitions.
