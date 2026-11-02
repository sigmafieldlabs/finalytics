# Datasets

Illustrative data for examples, tests and practice, read by `finalytics::datasets` in Rust,
`finalytics.datasets` in Python and `datasets` in JavaScript.

The data is synthetic: `scripts/datasets.py` draws it from fixed distributions with a fixed seed,
so that it gives the same figures everywhere. It is not the history of any market, index or fund,
and it redistributes no third party's data.

| File | What it holds |
| --- | --- |
| `asset-classes.csv` | Month-end levels, from October 1996 to October 2026, of six asset-class indices that start at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate. Their bear markets are modelled loosely on those of 2000–02, 2007–09, 2020 and 2022. |
| `portfolio/transactions.csv` | Three years of a two-account portfolio: deposits, purchases, sales, an FX conversion, dividends (with tax withheld on the European fund's), an advisory fee and a withdrawal. |
| `portfolio/instruments.csv` | Its five funds, with their currencies, asset classes and regions. |
| `portfolio/accounts.csv` | A taxable brokerage account and a tax-deferred retirement account. |
| `portfolio/prices.csv` | Daily prices of the funds, and the levels of a balanced benchmark, `BENCH`: 55% equity, 35% bonds and 10% real assets. |
| `portfolio/fx.csv` | The daily EUR/USD rate. |
| `portfolio/report-options.json` | The options of the portfolio's full report: grouping, the benchmark's segments for attribution, the risk-free rate and four scenarios. |

Each file also shows the layout Finalytics reads.
