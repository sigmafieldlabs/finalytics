---
description: Apply splits, spin-offs, mergers, symbol changes and returns of capital while preserving quantity and cost-basis accounting.
---

# Corporate actions

Corporate actions change existing lots in the account named by their transaction. They are records of real events, not adjustments to historical prices.

## Split

A split scales quantity by `to / from` and leaves cost unchanged. Record quoted, unadjusted prices on each date.

[split.py](../../examples/corporate-actions/split.py)

## Spin-off

A spin-off creates `ratio` new units per old unit and moves `cost_allocation` of each lot's cost to the new instrument. Acquisition dates carry across.

[spin-off.py](../../examples/corporate-actions/spin-off.py)

## Merger

A merger exchanges each old unit for `ratio` new units and optional `cash_per_unit`. Cash reduces basis; any amount beyond basis is realised.

[merger.py](../../examples/corporate-actions/merger.py)

## Symbol change

A symbol change moves the lots to a new identifier without resetting their cost or acquisition history. Both identifiers must use the same currency.

[symbol-change.py](../../examples/corporate-actions/symbol-change.py)

## Return of capital

A return of capital reduces basis in proportion to units held. Only a payment beyond remaining basis produces a realised gain.

[return-of-capital.py](../../examples/corporate-actions/return-of-capital.py)
