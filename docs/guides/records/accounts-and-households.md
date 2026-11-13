---
description: Combine taxable and non-taxable accounts into a household, group them by owner or wrapper, and reconcile account values.
---

# Accounts and households

Transactions name an account, defaulting to `main`. An accounts file adds names, lot methods, tax treatment and arbitrary `class.<scheme>` classifications.

## Account metadata

Taxable and non-taxable wrappers can share one portfolio. Classifications provide flexible groups without creating separate portfolios.

[account-metadata.py](../../examples/accounts-and-households/account-metadata.py)

## Household performance

The household view measures consolidated performance, each account's return and its contribution to the whole. A small account's high return need not contribute much.

[household.py](../../examples/accounts-and-households/household.py)

## Group by owner or wrapper

`group_by` aggregates account values, flows, returns and contributions for each classification. Accounts without a value are reported as unclassified.

[groups.py](../../examples/accounts-and-households/groups.py)

## Reconcile account series

Each account has its own value and external-flow series. Their sum is the household series; a transfer between two accounts nets to zero at household level.

[account-series.py](../../examples/accounts-and-households/account-series.py)

## Report on selected accounts

Pass account IDs to a report when you need one wrapper rather than the consolidated household. The example prints and shows the selected report.

[account-report.py](../../examples/accounts-and-households/account-report.py)
