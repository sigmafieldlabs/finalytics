<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![npm](https://img.shields.io/npm/v/finalytics)](https://www.npmjs.com/package/finalytics)
[![Downloads](https://img.shields.io/npm/dm/finalytics)](https://www.npmjs.com/package/finalytics)
[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#contributing-and-licence)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine written in Rust, run here as
WebAssembly in browsers, workers and Node.js, with TypeScript types included. It
compares portfolio optimisation methods on returns they have not seen, keeps the
books of a real portfolio to report its performance, risk and attribution, and
tests a plan's chance of meeting a goal.

```sh
npm install finalytics
```

Until 1.0.0 is released, `npm install finalytics` gives 0.2; install a 1.0
pre-release with `npm install finalytics@next`. 1.0 is a redesign of 0.2, as of
the Rust and Python 0.9 it bound: the
[migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md) explains what
replaces each part of 0.9. In Node.js the engine is ready on import; in a
browser, import the default export too and `await init()` once before the first
call.

**Sample reports**, built by the programs below from the bundled datasets:
[comparison](https://sigmafieldlabs.github.io/finalytics/reports/comparison.html) ·
[backtest](https://sigmafieldlabs.github.io/finalytics/reports/backtest.html) ·
[optimisation](https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html) ·
[portfolio](https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html) ·
[rebalance](https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html) ·
[planning](https://sigmafieldlabs.github.io/finalytics/reports/planning.html)

## Portfolio optimisation

Compare optimisation methods on the same assets. Each method is refitted every
quarter on the five years before, and judged on the quarter that follows.

```js
import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});

const comparison = assets.compare({
  methods: [
    "equal_weight",
    "minimum_variance",
    "maximum_sharpe",
    "maximum_diversification",
    "risk_parity",
    "hierarchical_risk_parity",
  ],
  max_weight: 0.4,
  rebalance: "quarterly",
  validation: { walk_forward: { years: 5 } },
  trading_cost_bps: 10,
});

comparison.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/comparison.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/comparison-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/comparison-light.png" alt="The comparison report: each method's return, volatility, Sharpe ratio, drawdown and turnover out of sample" width="760">
</picture></a>

Every result has the same shape: `result.report` is the report, which
`` `${result}` `` prints as text and `result.show()` opens as a page (in Node.js
in the default browser; in a browser, in a frame inside the element given), and
`result.results` holds every return and trade behind it.

To test one allocation, give a method or your own weights. Here, the maximum
Sharpe ratio fitted to the years before November 2016, held from then on against
a 60/40 benchmark:

```js
const backtest = assets.backtest({
  method: "maximum_sharpe",
  max_weight: 0.4,
  benchmark: { "US Equity": 0.6, Bonds: 0.4 },
  rebalance: "quarterly",
  validation: { holdout: { start: "2016-11-01" } },
});

backtest.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/backtest.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/backtest-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/backtest-light.png" alt="The backtest report: the allocation, its statistics in and out of sample against the benchmark, and its growth" width="760">
</picture></a>

To see the weights a method chooses today, where their risk comes from and
where they sit on the efficient frontier:

```js
const optimisation = assets.optimise({
  method: "maximum_sharpe",
  max_weight: 0.4,
  frontier: true,
});

optimisation.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/optimisation-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/optimisation-light.png" alt="The optimisation report: the weights, what they are expected to deliver, and each asset's share of the risk" width="760">
</picture></a>

## Portfolio analytics

Load a portfolio from its transactions and prices, and report on it: holdings,
performance, risk, attribution and scenarios.

```js
import { readFile } from "node:fs/promises";
import { Portfolio } from "finalytics";

const read = (file) => readFile(file, "utf8");
const portfolio = Portfolio.fromCsv({
  transactions: await read("transactions.csv"),
  prices: await read("prices.csv"),
  fx: await read("fx.csv"),
  instruments: await read("instruments.csv"),
  accounts: await read("accounts.csv"),
});

const report = portfolio.report({
  benchmark: "BENCH",
  group_by: ["asset_class", "region"],
});

report.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/portfolio-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/portfolio-light.png" alt="The portfolio report: value, returns, volatility, drawdown and value at risk, and growth against the benchmark" width="760">
</picture></a>

The files are the portfolio dataset: a household's brokerage and retirement
accounts, holding five funds in dollars and euros for three years.
`datasets.file("portfolio/prices.csv")` and the like return each of them, and
`datasets.portfolio()` loads the same portfolio without them. `BENCH` is a
column of `prices.csv`, the benchmark's levels; `asset_class` and `region` are
columns of `instruments.csv` that classify each holding. The sample report adds
the attribution and scenarios of `datasets.reportOptions()`. A portfolio's
report is its own result: every figure in it is the portfolio's, so `report()`
returns the report, with no `results` beside it.

<details>
<summary>The start of the dataset's files</summary>

```csv
id,kind,currency,name,class.asset_class,class.region
VTI,fund,USD,Vanguard Total Stock Market ETF,Equity,North America
EXSA,fund,EUR,iShares STOXX Europe 600 UCITS ETF (DE),Equity,Europe
```

```csv
date,VTI,EXSA,BND,GLD,VNQ,BENCH
2023-11-01,208.86,43.21,68.99,183.51,73.37,100.0000
2023-11-02,213.01,43.92,69.39,184.12,75.78,101.4345
```

```csv
id,name,lot_method,taxable,class.owner,class.wrapper
brokerage,Brokerage account,fifo,true,Alex,Taxable
ira,Retirement account,fifo,false,Alex,Tax-deferred
```

The [data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) describes every file, column by
column.

</details>

To move the same funds to their least risky mix, 5% to 40% each, optimise the
holdings and rebalance the accounts together, with fees and tax rates:

```js
const target = portfolio.assets().optimise({
  method: "minimum_variance",
  bounds: { min: 0.05, max: 0.4 },
});

const rebalancing = portfolio.rebalance(target, {
  whole_units: true,
  fee_bps: 5,
  minimum_fee: 1,
  tax: [0.37, 0.2],
});

rebalancing.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/rebalance-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/rebalance-light.png" alt="The rebalance report: the trades, how each holding's weight moves, and what the trades change and cost" width="760">
</picture></a>

Sales in the brokerage account take the lots that cost the least tax; the
retirement account is not taxable (`taxable` is false in `accounts.csv`), so its
sales pay none. `portfolio.withRebalance(rebalancing)` makes the trades.

## Financial Planning

Test a plan of saving, spending and investing against one goal: an amount by a
date, such as a house deposit, or payments over a period, such as an income in
retirement. The plan runs month by month through 10,000 markets drawn from the
assets' monthly returns, in today's money: returns are after inflation and
fees.

```js
import { Assets, datasets } from "finalytics";

const plan = new Assets(datasets.assetClassPrices()).plan({
  goal: { type: "payments", amount: 4000, start: "2046-11", end: "2076-10" },
  name: "Retirement income",
  chance: 0.85,
  start_value: 250000,
  flows: [{ type: "saving", amount: 1500, every: "month" }],
  weights: {
    type: "fixed",
    name: "Balanced 60/40",
    weights: { "US Equity": 0.6, Bonds: 0.4 },
  },
  alternatives: {
    "Cautious 30/70": { "US Equity": 0.3, Bonds: 0.7 },
    "Growth 80/20": { "US Equity": 0.8, Bonds: 0.2 },
  },
  inflation: 0.025,
  fee: 0.0025,
  currency: "USD",
});

plan.show();
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/planning.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/planning-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/planning-light.png" alt="The planning report: the plan, its chance of meeting the goal, and the money over time in simulated markets" width="760">
</picture></a>

The report gives the chance of meeting the goal against the chance wanted, how
the money could grow, and the smallest change that would reach that chance:
saving more, starting with more, starting the payments later or taking a
little less. It tests the plan when returns are lower and when markets crash as
the payments start, and compares other mixes on the same simulations.
`plan.results` holds the figures behind it, with the value over time.

For an amount by a date, give
`goal: { type: "target", amount: 150000, by: "2030-06" }`; for a single payment
out along the way, `{ type: "spending", name: "Car", amount: 25000, on: "2028-03" }`
among the flows; and for inflation month by month, a price index of one column,
such as a consumer price index, `inflation: index`.

## What is included

**Optimisation**

- Equal weight, minimum variance, mean–variance and robust mean–variance,
  maximum Sharpe, target return or volatility, maximum diversification, minimum
  CVaR, risk parity, hierarchical risk parity
- Limits on each asset and on groups, turnover, tracking error and factor
  exposure
- Black–Litterman and resampled estimates; Ledoit–Wolf, OAS and EWMA covariance
- Validation in sample, on a holdout, or walk-forward; calendar or drift
  rebalancing; trading costs

**Analytics**

- Books from transactions: lots, corporate actions, several accounts and
  currencies
- Time- and money-weighted returns, contribution, Brinson–Fachler and currency
  attribution
- Volatility, drawdowns, value at risk, risk decomposition, factor models and
  scenarios
- Rebalancing trades with fees and tax-aware lot selection, across accounts

**Planning**

- The chance of a goal, an amount by a date or payments over a period, from
  simulated markets: block bootstrap, normal or historical
- Saving and spending once or regularly, fixed or glide-path mixes, fees and
  inflation, in today's money
- The changes that reach the chance wanted, lower returns and a crash, and
  other mixes on the same simulations

Every report prints as text, 80 columns wide, and shows as one self-contained
HTML page, in light and dark. Every model is tested against an independent
implementation; the [methods pages](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) define each
one.

Beneath the front doors, functions such as `statistics`, `covariance`, `solve`,
`blackLitterman` and `brinsonFachler` give each method on its own. Data objects
use the field names of the [schema](https://github.com/sigmafieldlabs/finalytics/tree/main/schema); decimals are
strings, dates are ISO 8601 strings, and a series of numbers returned on its own
is a `Float64Array`.

## Off the main thread

`finalytics/client.js` runs the engine in a worker and gives the same calls,
returning promises. What comes back from the worker is data, without the methods
added on the main thread: a report has no `show` or `toText`, so lay it out with
`reportHtml` and `reportText`, as below:

```ts
import { createEngine } from "finalytics/client.js";

const engine = createEngine(
  new Worker(new URL("finalytics/worker.js", import.meta.url), {
    type: "module",
  }),
);
const portfolio = await engine.portfolioFromCsv({ transactions, prices });
const report = await portfolio.call("report", { benchmark: "BENCH" });
const html = await engine.call("reportHtml", report);
const text = await engine.call("reportText", report);
```

## Documentation

[Data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) ·
[Methods](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) ·
[Performance](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/performance.md) ·
[Migrating from 0.9](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md) ·
[Source and issues](https://github.com/sigmafieldlabs/finalytics)

## Contributing and licence

Contributions are welcome: see
[CONTRIBUTING.md](https://github.com/sigmafieldlabs/finalytics/blob/main/CONTRIBUTING.md). Finalytics is licensed under
[MIT](https://github.com/sigmafieldlabs/finalytics/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/sigmafieldlabs/finalytics/blob/main/LICENSE-APACHE),
at your option. It provides analysis tools only and is not investment advice.
