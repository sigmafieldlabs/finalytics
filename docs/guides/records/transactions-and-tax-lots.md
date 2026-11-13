---
description: Keep exact tax lots from transactions, choose relief methods, identify specific lots and inspect realised gains before trading.
---

# Transactions and tax lots

Every purchase opens a lot with its quantity, acquisition date and exact local and base cost. Sales close lots and record realised proceeds and gains. Purchase fees join cost; sale fees reduce proceeds.

## Realised gains by lot

This sale uses highest-cost relief across two purchases. Each gain names the opening transaction, so you can trace the calculation back to its lot.

[realised-gains.py](../../examples/transactions-and-tax-lots/realised-gains.py)

## Compare relief methods

An account's `lot_method` sets its default; a sale can override it. FIFO, LIFO, highest-cost and lowest-cost choose different lots and therefore different gains.

[relief-methods.py](../../examples/transactions-and-tax-lots/relief-methods.py)

## Average cost

Average-cost relief takes a proportional share of every open lot. Quantity and total cost remain exact across the closed and remaining parts.

[average-cost.py](../../examples/transactions-and-tax-lots/average-cost.py)

## Identify specific lots

Use `specific_id` and a `lots` field of `lot:quantity` pairs separated by semicolons. Lot identifiers are the IDs of the transactions that opened them.

[specific-lots.py](../../examples/transactions-and-tax-lots/specific-lots.py)

## Preview before selling

`lots_for_sale` applies a relief method without changing the books. Use it to review cost and lot choice before creating a sale or rebalancing proposal.

[preview.py](../../examples/transactions-and-tax-lots/preview.py)

> [!WARNING]
> **Long positions only:** Finalytics' transaction ledger and rebalancing currently support long positions only. Selling or transferring more units than an account holds raises an error.

For foreign-currency assets, realised gains measured in the base currency include both the asset's local price movement and FX movement between acquisition and disposal.
