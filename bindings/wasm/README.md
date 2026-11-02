<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![npm](https://img.shields.io/npm/v/finalytics)](https://www.npmjs.com/package/finalytics)
[![Downloads](https://img.shields.io/npm/dm/finalytics)](https://www.npmjs.com/package/finalytics)
[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#licence)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. This package runs the engine as WebAssembly in browsers, workers and Node.js, with
TypeScript types included.

> **Status:** 1.0 is in active development. Until 1.0.0 is released, `npm install finalytics`
> gives 0.2; install a 1.0 pre-release with `npm install finalytics@next`. Coming from 0.2? See
> the [migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md).

```sh
npm install finalytics
```

## Quick start

Estimate risk and return from a year of monthly returns, then find the portfolio with the highest
Sharpe ratio that holds between 10% and 60% of each asset. Saved as `quick-start.mjs`, it runs
with `node quick-start.mjs`:

```js
import { readFile } from "node:fs/promises";
import init, { covariance, solve } from "finalytics";

// In a browser, `await init()` loads the engine. In Node, pass it the WebAssembly file:
await init({ module_or_path: await readFile(new URL("finalytics_bg.wasm", import.meta.resolve("finalytics"))) });

// Twelve months of returns for three assets.
const returns = {
  labels: ["Equities", "Bonds", "Gold"],
  columns: [
    [-0.028, 0.021, -0.044, 0.043, -0.021, 0.020, 0.055, -0.028, -0.009, 0.017, 0.044, 0.014],
    [-0.009, 0.020, 0.002, 0.021, -0.013, 0.005, 0.014, 0.002, -0.001, -0.011, -0.028, 0.032],
    [-0.013, -0.029, 0.004, 0.069, -0.009, 0.000, 0.017, 0.005, -0.030, 0.003, 0.057, -0.001],
  ],
};

// Annualised estimates of risk and return.
const { matrix } = covariance(returns, { method: "ledoit_wolf" });
const annualised = { ...matrix, values: matrix.values.map((v) => v * 12) };
const expected = returns.columns.map((r) => (12 * r.reduce((a, b) => a + b, 0)) / r.length);

// The portfolio with the highest Sharpe ratio, holding 10% to 60% of each asset.
const solution = solve(
  { labels: returns.labels, covariance: annualised, expected_returns: expected, constraints: { bounds: { min: 0.1, max: 0.6 } } },
  { objective: "maximum_sharpe", risk_free_rate: 0.02 },
);
const weights = solution.labels.map((asset, i) => `${asset} ${solution.weights[i].toFixed(3)}`);
console.log(weights.join(", ")); // Equities 0.444, Bonds 0.180, Gold 0.375
console.log(solution.expected_return.toFixed(4), solution.volatility.toFixed(4)); // 0.0709 0.0670
```

The same example is in the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#quick-start) and the
[Python package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/python#quick-start), and it runs as a test, so the
results shown are the engine's.

## Books, performance and reports

```ts
const books = new Books(portfolioFromCsv(instrumentsCsv, transactionsCsv, "USD"), MarketData.fromCsv(pricesCsv));
const performance = books.performance();
performance.period("YTD");
const report = books.performanceReport({ benchmark: "SPY" });
const tables = reportTables(report); // for CSV, spreadsheet or PDF export
```

Data objects use the field names of the
[schema](https://github.com/sigmafieldlabs/finalytics/tree/main/schema) (`sharpe_ratio`,
`expected_return`); decimals are strings, dates are ISO 8601 strings, and series of numbers in
data objects are arrays. A series of numbers returned on its own, such as `performance.returns`
or `drawdownSeries(...)`, is a `Float64Array`. Invalid input throws an `Error` named
`ValidationError` whose `issues` list every problem with where it is.

## Off the main thread

```ts
import { createEngine } from "finalytics/client.js";

const engine = createEngine(
  new Worker(new URL("finalytics/worker.js", import.meta.url), { type: "module" }),
);
const stats = await engine.call("statistics", returns);
const books = await engine.books(portfolio, market.toDocument());
const performance = await books.call("performance");
const daily = await performance.get("returns");
```

- Source and issues: https://github.com/sigmafieldlabs/finalytics
- Licence: MIT OR Apache-2.0
