---
description: Load the bundled household portfolio and produce a report of its holdings, performance and risk against a benchmark.
---

# Your first portfolio

A Finalytics portfolio keeps transactions, market prices, exchange rates, instrument details
and accounts together. Its report turns them into one view of what you hold, how it performed
and where its risk comes from.

## Start with the bundled household

`datasets.portfolio()` loads a household's brokerage and retirement accounts. It holds five
real funds in dollars and euros from November 2023 to October 2026, with illustrative
transactions and a balanced benchmark named `BENCH`.

This example reports on the household against that benchmark and groups the holdings by
asset class and region. Run it, then switch between the text and report views of the output:

[report.py](../../examples/your-first-portfolio/report.py)

## Read the report

Start with the portfolio's value and allocation, then read its performance against `BENCH`.
The risk sections put returns beside volatility, drawdown and tail risk. Grouping shows which
asset classes and regions account for the holdings rather than treating every fund as an
isolated position.

Printing a portfolio report returns its text. `show()` produces the interactive HTML report;
in a notebook it appears inline, and on this site it appears in the report view. Unlike an
optimisation or backtest, a portfolio report is itself the result: it has no separate
`results` object beside it.

> [!NOTE]
> The five funds' prices are real, but the household and its transactions are illustrative.
> Use the dataset to learn the workflow, not as an investment recommendation.

## Use your own portfolio

Your own data takes the same shape: transactions, prices, exchange rates, instruments and
accounts. Load the CSV files with `Portfolio.from_csv` in Python, `Portfolio.fromCsv` in
JavaScript or `Portfolio::builder()` in Rust. `BENCH` is a price column, while `asset_class`
and `region` are instrument classifications in this example.

The [data guide](../../data.md) describes every file and column. The dataset helpers can also
return the bundled CSV text when you want a template to adapt.

## Next steps

An optimisation starts from the portfolio's assets rather than its report. Use
`portfolio.assets()` to estimate their returns and covariance, choose weights, and then
rebalance the accounts toward them. The next guide begins with the simpler asset-class
dataset so you can focus on the optimisation itself.
