# Finalytics for JavaScript and TypeScript

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. This package runs the engine as WebAssembly in browsers, workers and Node.js, with
TypeScript types included.

> **Status:** 1.0 is in active development (`1.0.0-alpha`).

```sh
npm install finalytics
```

```ts
import init, { Books, MarketData, portfolioFromCsv, statistics, solve } from "finalytics";

await init(); // in Node: await init({ module_or_path: await readFile(wasmPath) })

const portfolio = portfolioFromCsv(instrumentsCsv, transactionsCsv, "USD");
const market = MarketData.fromCsv(pricesCsv, fxCsv);
const books = new Books(portfolio, market);

const performance = books.performance();
performance.period("YTD");
statistics(performance.returns, { risk_free_rate: 0.03 });

const solution = solve(
  { labels, covariance, expected_returns: mu },
  { objective: "maximum_sharpe", risk_free_rate: 0.03 },
);
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
