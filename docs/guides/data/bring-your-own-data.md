---
description: Map broker and market-data exports to Finalytics CSV files, choose wide or long prices, and run the examples on local files.
files: [transactions.csv, prices.csv, fx.csv]
---

# Bring your own data

A portfolio needs only `transactions`; add `prices` to value its instruments, `fx` for other currencies, and optional `instruments`, `accounts` and `goals` files. CSV headers are case-insensitive, dates use ISO form, and decimal numbers use a dot without thousands separators.

## Start with the minimum

A deposit provides opening cash and establishes the base currency. Purchases name an instrument, quantity and price. The price file can have one column per instrument.

[minimal.py](../../examples/bring-your-own-data/minimal.py)

## Wide prices for Assets

`Assets.from_csv` accepts a `date` column followed by one total-return price column per asset. Monthly dates imply 12 periods a year; you can override frequency when constructing `Assets` directly.

[wide-prices.py](../../examples/bring-your-own-data/wide-prices.py)

> [!WARNING]
> Use adjusted, income-inclusive prices for `Assets`. For a transaction-based `Portfolio`, use quoted prices without income and record dividends and splits as transactions, or you will count them twice.

## Long prices for Portfolio

A portfolio's prices also accept `date`, `instrument` and `price` columns. This layout is convenient when each instrument has a different observation calendar.

[long-prices.py](../../examples/bring-your-own-data/long-prices.py)

## Add names, classifications and account rules

Instrument `class.<scheme>` columns drive grouping in reports and contribution. Account metadata controls display names, tax treatment, lot relief and classifications such as owner.

[metadata.py](../../examples/bring-your-own-data/metadata.py)

## Use local files

On your computer, ordinary file reads work as in any script. On this page, choose or drop `transactions.csv`, `prices.csv` and `fx.csv` below the example; they stay in your browser and are mapped to the same filenames for Python and JavaScript. Nothing is uploaded to a server.

[local-files.py](../../examples/bring-your-own-data/local-files.py)

The example uses the packaged household files until you choose replacements. Continue to **Validate data and read errors** before relying on an imported export.
