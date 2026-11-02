// Checks the WebAssembly performance budget (see docs/performance.md): building the books for
// 100,000 transactions over 10 years for 2,000 instruments in under 2 seconds.
//
//   cargo run --release -p finalytics-bench --bin budgets -- --write-document large.json
//   node bindings/wasm/test/budget.mjs large.json
//
// The time measured is `new Portfolio(portfolio, market)` from JavaScript objects, as an application
// would call it: it includes reading the 100,000 transactions out of JavaScript. Each
// measurement is the median of five runs.
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";

import { engine } from "./setup.mjs";

const BUDGET_SECONDS = 2;
const path = process.argv[2];
if (!path) {
  console.error("usage: node budget.mjs <portfolio document>");
  process.exit(2);
}

const fl = await engine();
const document = fl.readDocument(await readFile(path, "utf8"));
const times = [];
for (let run = 0; run < 5; run++) {
  const started = performance.now();
  const books = new fl.Portfolio(document.portfolio, document.market);
  times.push((performance.now() - started) / 1000);
  books.free();
}
times.sort((a, b) => a - b);
const median = times[2];
const transactions = document.portfolio.transactions.length;
console.log(
  `books for ${transactions} transactions in WebAssembly: ${median.toFixed(3)} s ` +
    `(${median <= BUDGET_SECONDS ? "within" : "OVER"} the budget of ${BUDGET_SECONDS} s)`,
);
process.exit(median <= BUDGET_SECONDS ? 0 : 1);
