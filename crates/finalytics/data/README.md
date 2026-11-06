# Datasets

Data for examples, tests and practice, read by `finalytics::datasets` in Rust,
`finalytics.datasets` in Python and `datasets` in JavaScript. Each file also shows the layout
Finalytics reads.

## A household portfolio

Three years, from November 2023 to October 2026, of a taxable brokerage account and a
tax-deferred retirement account held by one person. The funds, their prices, their distributions
and the EUR/USD rate are real; the transactions are illustrative.

| File | What it holds |
| --- | --- |
| `portfolio/transactions.csv` | Deposits, purchases, a sale, an FX conversion, the funds' distributions as dividends (with 15% tax withheld on the European fund's), an advisory fee and a withdrawal. |
| `portfolio/instruments.csv` | Its five funds: Vanguard Total Stock Market (`VTI`), iShares STOXX Europe 600 (`EXSA`, in euros on Xetra), Vanguard Total Bond Market (`BND`), SPDR Gold Shares (`GLD`) and Vanguard Real Estate (`VNQ`), with their currencies, asset classes and regions. |
| `portfolio/accounts.csv` | The two accounts, with their owner and tax wrapper; the retirement account is not taxable. |
| `portfolio/prices.csv` | Daily closes of the funds on US trading days (for `EXSA`, the last Xetra close), and `BENCH`: a balanced benchmark built from the funds' total returns, 55% equity, 35% bonds and 10% real assets, rebalanced monthly. |
| `portfolio/fx.csv` | The daily EUR/USD rate. |
| `portfolio/report-options.json` | The options of the portfolio's full report: grouping, the benchmark's segments for attribution, the T-bills' return over the period as the risk-free rate, and four scenarios. |

## Six asset classes

| File | What it holds |
| --- | --- |
| `asset-classes.csv` | Month-end levels, from October 1996 to October 2026, of six asset-class indices that start at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate. The data is synthetic: drawn from fixed distributions with a fixed seed, with bear markets modelled loosely on those of 2000–02, 2007–09, 2020 and 2022. |

## Sources

- The funds' prices and distributions, the EUR/USD rate, and the 13-week Treasury bill rate
  (`^IRX`) behind the risk-free rate of the report options: Yahoo Finance, downloaded by
  `scripts/fetch-market-data.py`. The fund data is kept in `scripts/portfolio-market.json`.
- The household's transactions and the asset-class indices: `scripts/datasets.py`.

The data is provided for examples and education, as it is, with no warranty of accuracy. The
prices remain the property of their providers.
