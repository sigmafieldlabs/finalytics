# Data guide

Finalytics reads two kinds of input:

- **the prices or returns of assets**, for `Assets`: statistics, covariance and optimisation;
- **a portfolio's records**, for `Portfolio`: its transactions and, optionally, its instruments
  and accounts, valued with prices and exchange rates.

This guide describes each input, column by column. Every file shown here is one of the
[datasets](../crates/finalytics/data) that come with the library, so you can open a complete
example of each layout with `fl.datasets.file(name)` in Python, `datasets.file(name)` in
JavaScript or `finalytics::datasets::file(name)` in Rust.

Files are CSV with a header row; column names are matched without regard to case or surrounding
spaces. Dates are ISO 8601 (`2026-11-02`), and numbers use a dot for decimals and no thousands
separators. Every problem in every file is reported at once, by file, line and column, such as
`transactions.line 15 (quantity)`.

## Prices for `Assets`

A `date` column and one column of prices per asset:

```python
>>> import finalytics as fl
>>> print("\n".join(fl.datasets.file("asset-classes.csv").splitlines()[:3]))
date,US Equity,Intl Equity,Bonds,T-Bills,Gold,Real Estate
1996-10-31,100.0000,100.0000,100.0000,100.0000,100.0000,100.0000
1996-11-29,95.6321,98.3068,100.3593,100.4102,102.1241,90.6481

```

The same prices may come as a pandas DataFrame indexed by date, with one column per asset, or in
the long layout of `date`, `instrument` and `price` columns. The assets keep the order of the
columns.

- **Prices should include income.** Use total-return prices, such as adjusted closes, so that
  dividends and interest count in each asset's return.
- **Dates with a missing price are left out**, with a warning, so that every asset's returns
  cover the same periods.
- **The frequency comes from the dates.** The median gap between dates gives the number of
  periods in a year that annualises the figures: up to 4 days is daily (252), up to 10 weekly
  (52), up to 45 monthly (12), up to 135 quarterly (4), and longer yearly (1). Give
  `periods_per_year` to override it.

```python
>>> assets = fl.Assets(fl.datasets.asset_class_prices())
>>> assets.periods_per_year, len(assets.dates), assets.labels[:2]
(12.0, 360, ['US Equity', 'Intl Equity'])

```

Returns instead of prices go to `Assets.from_returns`: a DataFrame indexed by the date each
return ends on, or an array with its dates and labels.

## A portfolio's files

`Portfolio.from_csv` takes up to six files; only the transactions are required.

| File | Required | Holds |
| --- | --- | --- |
| `transactions` | yes | Every deposit, purchase, sale, dividend, fee and other transaction |
| `prices` | to value instruments | The price of each instrument on each date, and of any benchmark |
| `fx` | for other currencies | Exchange rates for instruments and cash not in the base currency |
| `instruments` | no | Each instrument's kind, currency, name and classifications |
| `accounts` | no | Each account's name, lot method and classifications |
| `goals` | no | Goals kept with the portfolio, for planning |

### Transactions

Each row has a `date` (or `trade_date`) and a `type`, and the columns its type needs:

| Type | Columns |
| --- | --- |
| `deposit`, `withdrawal`, `fee`, `tax` | `amount`, `currency` |
| `buy` | `instrument`, `quantity`, `price`, optional `fees` |
| `sell` | `instrument`, `quantity`, `price`, optional `fees`, `lot_method`, `lots` |
| `dividend` | `instrument`, `amount`, `currency`, optional `withholding_tax` |
| `interest` | optional `instrument`, `amount`, `currency`, optional `withholding_tax` |
| `fx_conversion` | `sell_amount`, `sell_currency`, `buy_amount`, `buy_currency` |
| `transfer_in` | `instrument`, `quantity`, optional `cost_basis`, `acquired` |
| `transfer_out` | `instrument`, `quantity`, optional `lot_method`, `lots` |
| `split` | `instrument`, `from`, `to` |
| `spin_off` | `instrument`, `new_instrument`, `ratio`, `cost_allocation` |
| `merger` | `instrument`, `new_instrument` (unless all cash), `ratio`, optional `cash_per_unit` |
| `symbol_change` | `instrument`, `new_instrument` |
| `return_of_capital` | `instrument`, `amount`, `currency` |

Every row may also give:

- `id`: the transaction's identifier; `t1`, `t2` and so on, by row, when left out;
- `account`: the account; `main` when left out;
- `settle_date` and `note`.

Amounts are positive; the type says which way they go. A purchase's price and fees are in the
instrument's currency, and the cash for them comes from that currency's balance: buy euros with
an `fx_conversion` before buying a fund priced in euros. Fees on a trade count in the holding's
cost and return; a `fee` or `tax` charged to the account is an expense of the portfolio, shown on
its own line in contribution. `lots` names the lots to sell for specific identification, as
`lot:quantity` pairs separated by semicolons (`t1:5; t3:2.5`), where a lot is the identifier of
the transaction that opened it.

```python
>>> print("\n".join(fl.datasets.file("portfolio/transactions.csv").splitlines()[:5]))
id,account,trade_date,type,instrument,quantity,price,fees,amount,currency,sell_amount,sell_currency,buy_amount,buy_currency,withholding_tax,note
t1,brokerage,2023-11-01,deposit,,,,,250000,USD,,,,,,
t2,ira,2023-11-01,deposit,,,,,150000,USD,,,,,,
t3,brokerage,2023-11-01,buy,VTI,478,208.86,1,,,,,,,,
t4,brokerage,2023-11-01,fx_conversion,,,,,,,37485.14,USD,35433.20,EUR,,

```

A file needs only the columns its rows use. This is a whole portfolio:

```python
>>> portfolio = fl.Portfolio.from_csv(
...     transactions=(
...         "date,type,instrument,quantity,price,amount,currency\n"
...         "2026-01-02,deposit,,,,20000,USD\n"
...         "2026-01-02,buy,VOO,20,500,,\n"
...         "2026-01-02,buy,AGG,100,98,,\n"
...     ),
...     prices="date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n",
... )
>>> portfolio.base_currency, str(portfolio.valuation().total_base)
('USD', '20120.0')

```

### Prices

A `date` column and one column per instrument, named by its identifier, or the long layout of
`date`, `instrument` and `price` (or `close`) columns. Prices are in each instrument's currency.

- **Use prices without income.** A portfolio's dividends and interest are its transactions, so
  prices that include them would count them twice. For the same reason, a split is a `split`
  transaction: use prices as quoted on each date, not adjusted for later splits.
- **A missing price is carried forward** from an earlier date for up to five days, with a
  warning; beyond that, a valuation that needs it is an error.
- **A benchmark is a price column too**, such as an index's levels. It need not be held: name it
  in the report options (`benchmark="BENCH"`).

### Exchange rates

A `date` column and one column per currency pair, named `EUR/USD`, `EURUSD` or `EUR.USD`, or the
long layout of `date`, `base`, `quote` and `rate` columns. A rate is the number of units of the
second currency per unit of the first. A rate is also found inverted, or crossed through the US
dollar, so each pair needs giving once.

### Instruments

| Column | Required | Holds |
| --- | --- | --- |
| `id` | yes | The identifier the transactions and prices use |
| `kind` | yes | `equity`, `fund` (or `etf`), `bond`, `future`, `option`, `fx_forward` or `other` |
| `currency` | yes | The currency of its prices |
| `name` | no | A name to show in reports |
| `multiplier` | no | Units of value per unit of price, for futures and options (1 by default) |
| `class.<scheme>` | no | Classifications to group by, such as `class.asset_class` or `class.region` |
| `id.<scheme>` | no | Other identifiers, such as `id.isin` |

Without an instruments file, each instrument the transactions name has kind `other` and the
currency given with it in the transactions (by a dividend, say), or else the base currency.

```python
>>> print(fl.datasets.file("portfolio/instruments.csv").splitlines()[1])
VTI,fund,USD,Vanguard Total Stock Market ETF,Equity,North America

```

### Accounts

`id`, with optional `name`, `lot_method`, `taxable` and `class.<scheme>` columns such as
`class.owner`. The lot method chooses the lots a sale closes: `fifo` (the default), `lifo`,
`highest_cost`, `lowest_cost`, `average_cost` or `specific_id`. `taxable` is `true` by default;
`false` marks a tax-deferred or tax-exempt account, such as a retirement account, whose sales
rebalancing does not tax. Without an accounts file, each account the transactions name is
created taxable, with first in, first out.

### Goals

Goals kept with the portfolio, one record per row in the style of the transactions file: `goal`,
the goal's name, which every row of it repeats, and `type`.

| Type | Columns |
| --- | --- |
| `target` | `amount` by the month `by` |
| `payments` | `amount` every month (or `every` `quarter` or `year`) from `start` to `end` |
| `projection` | to the month `end`, with no goal |
| `saving`, `spending` | `amount`, once `on` a month or `every` month, quarter or year from `start` to `end`; `name` and `growth` if given |
| `mix`, `alternative` | the plan's `weights`, or another mix's to compare, written as `US Equity=0.6;Bonds=0.4`, with `name` |
| `glide`, `alternative_glide` | one `month` of a glide path and its `weights`; the rows of each path share its `name` |

Each goal has one `target`, `payments` or `projection` row, which also takes the plan's settings:
`chance` and `spare`, `start_value`, `plan_start` (the first month), `inflation` (a rate a
year), `fee`, `model` (`bootstrap`, with `block`, `normal` or `historical`), `simulations` and
`seed`. Months are written `2030-06`. Each goal is a plan, checked when it is planned with the
assets it names: `assets.plan(portfolio.goals[0])`.

```text
goal,type,amount,by,every,name,weights,start_value,inflation
House,target,150000,2030-06,,,,60000,0.025
House,saving,2500,,month,,,,
House,mix,,,,Balanced,US Equity=0.6;Bonds=0.4,,
```

### Base currency

The currency results are reported in. Without one, it is the currency of the first deposit, or
of the first transaction with a currency.

## The portfolio document

A portfolio and its market data can be kept as one JSON document,
`{"format": "finalytics.portfolio", "format_version": 1, "portfolio": ..., "market": ...}`,
described by [`schema/portfolio-document.schema.json`](../schema/portfolio-document.schema.json).
`portfolio.to_json()` writes one and `Portfolio.read(path)` reads it, reporting where a malformed
document goes wrong. Goals kept with the portfolio are its `goals`, each a plan with its name;
a document without goals has no `goals` member, so it reads and digests as before.

## Report options

The reports take their options as keywords (Python), an options object (JavaScript) or a
`ReportOptions` (Rust), or as JSON. The portfolio dataset's are an example of every kind of
option, with the benchmark's segments for attribution and four scenarios:

```python
>>> options = fl.datasets.report_options()
>>> options.benchmark, options.group_by, len(options.scenarios)
('BENCH', ['asset_class', 'region'], 4)

```

## Bringing your own data

- **From a broker.** Map the export's columns to the transaction layout above, one row per
  transaction; a pandas DataFrame written with `to_csv` is fine. Keep the broker's identifiers as
  transaction `id`s so that a sale can name the lots it closes.
- **From a data provider.** For `Assets`, use adjusted closes, which include income. For a
  `Portfolio`, use closes as quoted, and record dividends, splits and other corporate actions as
  transactions.
- **A long history.** Start the transactions with the holdings on the first date, as
  `transfer_in` rows with their cost basis and acquisition dates, rather than replaying every
  trade since the account opened.
- **Check before you report.** `PortfolioData.validate()` lists every structural problem without
  applying the transactions, and building a `Portfolio` reports a transaction the books cannot
  apply, such as a sale of more than is held.
