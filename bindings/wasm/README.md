# Finalytics for JavaScript and TypeScript

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. This package runs the engine as WebAssembly in browsers, workers and Node.js, with
TypeScript types included.

> **Status:** 1.0 is in active development. Until 1.0.0 is released, `npm install finalytics`
> gives 0.2; install a 1.0 pre-release with `npm install finalytics@next`.

```sh
npm install finalytics
```

## Quick start

A portfolio is its instruments and transactions; with prices, the engine keeps its books and
measures it. Here, a deposit buys two funds at the end of January, and their prices at the end of
each month value the portfolio. Saved as `quick-start.mjs`, it runs with `node quick-start.mjs`:

```js
import { readFile } from "node:fs/promises";
import init, { Books, MarketData, portfolioFromCsv, statistics } from "finalytics";

// In a browser, `await init()` loads the engine. In Node, pass it the WebAssembly file:
const wasm = new URL("finalytics_bg.wasm", import.meta.resolve("finalytics"));
await init({ module_or_path: await readFile(wasm) });

const portfolio = portfolioFromCsv(
  `id,kind,currency,class.asset_class
SPY,fund,USD,Equity
AGG,fund,USD,Bonds
`,
  `id,account,trade_date,type,instrument,quantity,price,amount,currency
t1,main,2026-01-30,deposit,,,,10000,USD
t2,main,2026-01-30,buy,SPY,12,500,,
t3,main,2026-01-30,buy,AGG,40,100,,
`,
  "USD",
);
const market = MarketData.fromCsv(`date,SPY,AGG
2026-01-30,500,100
2026-02-27,520,100.8
2026-03-31,490,101.5
2026-04-30,530,101
`);
const books = new Books(portfolio, market);

// The value at the end of April, in the base currency.
console.log(books.valuation("2026-04-30").total_base); // 10400

// The time-weighted return since the purchase, each holding's contribution to it, and
// statistics of the monthly returns.
const performance = books.performance();
console.log(performance.period("ITD").cumulative.toFixed(4)); // 0.0400
const contributions = performance.contributions().map((c) => `${c.holding.id} ${c.contribution.toFixed(4)}`);
console.log(contributions.join(", ")); // AGG 0.0040, SPY 0.0360, USD 0.0000
const stats = statistics(performance.returns, { periods_per_year: 12 });
console.log(stats.volatility.toFixed(4), stats.max_drawdown.toFixed(4)); // 0.1420 -0.0323
```

The same example is in the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#quick-start) and the
[Python package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/python#quick-start).

## More of the API

```ts
const solution = solve(
  { labels, covariance, expected_returns: mu },
  { objective: "maximum_sharpe", risk_free_rate: 0.03 },
);
const report = books.performanceReport({ benchmark: "SPY" });
const tables = reportTables(report); // for CSV, spreadsheet or PDF export
```

Data objects use the field names of the
[schema](https://github.com/sigmafieldlabs/finalytics/tree/main/schema) (`sharpe_ratio`,
`expected_return`); decimals are strings, dates are ISO 8601 strings, and series of numbers are
`Float64Array`s. Invalid input throws an `Error` named `ValidationError` whose `issues` list every
problem with where it is.

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
