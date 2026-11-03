<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![npm](https://img.shields.io/npm/v/finalytics)](https://www.npmjs.com/package/finalytics)
[![Downloads](https://img.shields.io/npm/dm/finalytics)](https://www.npmjs.com/package/finalytics)
[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#licence)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine: it keeps the books of a
multi-currency portfolio from its transactions, measures performance and risk,
explains returns against a benchmark, and optimises portfolios under real-world
constraints. This package runs the engine as WebAssembly in browsers, workers
and Node.js, with TypeScript types included.

> **Status:** 1.0 is in active development. Until 1.0.0 is released,
> `npm install finalytics` gives 0.2; install a 1.0 pre-release with
> `npm install finalytics@next`. Coming from 0.2? See the
> [migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md).

```sh
npm install finalytics
```

In Node.js the engine is ready on import. In a browser, import the default
export too and `await init()` once before the first call.

## A portfolio in four steps

Review a portfolio, find where its risk comes from, choose a better mix of the
same funds, and get the trades that reach it, in each account. The portfolio is
one of the datasets that come with the package: three years of a brokerage and a
retirement account, in dollars and euros. Saved as `workflow.mjs`, it runs with
`node workflow.mjs`:

```js
import { writeFile } from "node:fs/promises";
import { datasets, reportHtml } from "finalytics";

const pct = (x, digits = 1) => `${(100 * x).toFixed(digits)}%`;
const signed = (x) => (x < 0 ? "" : "+") + pct(x);

// Three years of a two-account portfolio, in dollars and euros.
const portfolio = datasets.portfolio();

// 1. Review: every report, from one set of options.
const report = portfolio.report({
  benchmark: "BENCH",
  group_by: ["asset_class", "region"],
  risk_free_rate: 0.03,
});
await writeFile("report.html", reportHtml(report, "Morgan family portfolio"));
const since = report.performance.periods.at(-1);
console.log(
  `Return ${signed(since.cumulative)}, benchmark ${signed(since.benchmark)}`,
);

// 2. Diagnose: which holdings carry the risk?
for (const part of report.risk.holdings.decomposition.contributions) {
  if (part.share > 0.1) {
    console.log(
      `${part.label}: ${pct(part.weight, 0)} of value, ` +
        `${pct(part.share, 0)} of risk`,
    );
  }
}

// 3. Decide: the least risky mix of the same funds, 5% to 40% each.
const target = portfolio.assets().optimise({
  method: "minimum_variance",
  bounds: { min: 0.05, max: 0.4 },
});
const mixes = [["Now", report.overview.current], ["After", target]];
for (const [name, mix] of mixes) {
  console.log(
    `${name}: volatility ${pct(mix.volatility)}, ` +
      `expected return ${pct(mix.expected_return)}`,
  );
}

// 4. Trade: what each account buys and sells, in whole units.
const proposal = portfolio.rebalance(target.weights, { whole_units: true });
for (const account of proposal.accounts) {
  for (const trade of account.trades) {
    console.log(account.account, trade.side, trade.quantity, trade.instrument);
  }
}
```

It prints:

```text
Return +56.1%, benchmark +55.2%
VTI: 51% of value, 79% of risk
Now: volatility 9.5%, expected return 14.3%
After: volatility 7.5%, expected return 11.2%
brokerage sell 394 VTI
brokerage buy 898 BND
brokerage buy 1024 EXSA
brokerage buy 30 GLD
brokerage buy 200 VNQ
ira sell 259 VTI
ira buy 571 BND
ira buy 651 EXSA
ira buy 19 GLD
ira buy 127 VNQ
```

`report.html` is one self-contained page: every report, with charts, in light
and dark, ready to print. `reportText(report)` gives the same reports as text,
80 columns wide.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/report-overview-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/report-overview.png" alt="The overview of the HTML report: value, returns, volatility, drawdown and value at risk, and the growth of 100 against the benchmark" width="760">
</picture>

Give the report options the benchmark's segments and some scenarios, as
`datasets.reportOptions()` does, and the report adds attribution against the
benchmark and the effect of each scenario.

## Your own portfolio

A portfolio needs its transactions and the prices that value them:

```js
import { readFile } from "node:fs/promises";
import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: await readFile("transactions.csv", "utf8"),
  prices: await readFile("prices.csv", "utf8"),
});
console.log(portfolio.valuation().total_base);
```

```csv
date,type,instrument,quantity,price,amount,currency
2026-01-02,deposit,,,,20000,USD
2026-01-02,buy,VOO,20,500,,
2026-01-02,buy,AGG,100,98,,
```

Add `fx` for other currencies, `instruments` to classify holdings by asset class
or region, and `accounts` to name accounts. The
[data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md)
describes every file, column by column, and
`datasets.file("portfolio/transactions.csv")` returns a complete example of
each. Invalid input throws an `Error` named `ValidationError` whose `issues`
list every problem with its file, line and column.

## From prices to a portfolio

`Assets` takes prices or returns for statistics, covariance and optimisation.
Here, the mix of six asset classes with the highest Sharpe ratio, with at most
40% in any one and 15% in real assets, against the return on T-bills:

```js
import { Assets, datasets } from "finalytics";

// Thirty years of month-end levels of six asset-class indices.
const prices = datasets.assetClassPrices();

const assets = new Assets(prices, { risk_free_asset: "T-Bills" });
const result = assets.optimise({
  method: "maximum_sharpe",
  bounds: { min: 0, max: 0.4 },
  groups: {
    "Real assets": { assets: ["Gold", "Real Estate"], min: 0, max: 0.15 },
  },
});
for (const [asset, weight] of Object.entries(result.weights)) {
  const percent = (100 * weight).toFixed(0).padStart(3);
  console.log(`${asset.padEnd(12)} ${percent}%`);
}
console.log(`Sharpe ratio ${result.sharpe_ratio.toFixed(2)}`);
```

```text
US Equity     40%
Intl Equity    5%
Bonds         40%
Gold          15%
Real Estate    0%
Sharpe ratio 0.55
```

The other objectives are `minimum_variance`, `mean_variance`,
`robust_mean_variance`, `target_return`, `target_volatility`,
`maximum_diversification`, `minimum_cvar`, `risk_budgeting` and
`hierarchical_risk_parity`; `assets.frontier(20)` gives the efficient frontier.

## Beneath the front doors

Functions such as `statistics`, `covariance`, `riskDecomposition`, `solve`,
`blackLitterman` and `brinsonFachler` give each method on its own. The
[methods pages](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods)
define every model, with examples.

Data objects use the field names of the
[schema](https://github.com/sigmafieldlabs/finalytics/tree/main/schema)
(`sharpe_ratio`, `expected_return`); decimals are strings, dates are ISO 8601
strings, and series of numbers in data objects are arrays. A series of numbers
returned on its own, such as `performance.returns` or `drawdownSeries(...)`, is
a `Float64Array`. `reportTables(report)` gives any report as typed tables, for
CSV, spreadsheet or PDF export.

## Off the main thread

`finalytics/client.js` runs the engine in a worker and gives the same calls,
returning promises:

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
```

- Source and issues: https://github.com/sigmafieldlabs/finalytics
- Licence: MIT OR Apache-2.0
