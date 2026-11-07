// Checks the WebAssembly performance budgets (see docs/performance.md):
//
// - building the books for 100,000 transactions over 10 years for 2,000 instruments in under
//   2 seconds;
// - a screen of every measure on 5,000 securities with up to 10 years of daily closes, volumes
//   and dividends (about 12 million closes) in under 1 second.
//
//   cargo run --release -p finalytics-bench --bin budgets -- --write-document large.json
//   node bindings/wasm/test/budget.mjs large.json
//
// The books are timed as `new Portfolio(portfolio, market)` from JavaScript objects, as an
// application would call it: it includes reading the 100,000 transactions out of JavaScript. The
// screen is timed as `universe.screen()` on a universe built once with `Universe.fromColumns`
// from typed arrays generated here from a fixed seed; the time to build it is shown too. Each
// measurement is the median of five runs. Without a document, only the screen is checked.
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";

import { engine } from "./setup.mjs";

const fl = await engine();
let failed = false;

function median(run) {
  const times = [];
  for (let k = 0; k < 5; k++) {
    const started = performance.now();
    run();
    times.push((performance.now() - started) / 1000);
  }
  times.sort((a, b) => a - b);
  return times[2];
}

function report(what, seconds, budget) {
  const within = seconds <= budget;
  console.log(`${what}: ${seconds.toFixed(3)} s (${within ? "within" : "OVER"} the budget of ${budget} s)`);
  failed ||= !within;
}

const path = process.argv[2];
if (path) {
  const document = fl.readDocument(await readFile(path, "utf8"));
  const seconds = median(() => new fl.Portfolio(document.portfolio, document.market).free());
  report(`books for ${document.portfolio.transactions.length} transactions in WebAssembly`, seconds, 2);
}

/**
 * The screening budget's universe: `securities` random walks with drift over up to `years` years
 * of business days to 30 October 2026, each starting up to a tenth of the way in, with volumes
 * and quarterly dividends; the first, `INDEX`, is the benchmark.
 */
function largeUniverse(securities, years) {
  let seed = 5000;
  const uniform = () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return (seed >>> 0) / 4294967296;
  };
  const normal = () => Math.sqrt(-2 * Math.log(Math.max(uniform(), 1e-300))) * Math.cos(2 * Math.PI * uniform());
  const rows = years * 252;
  const days = new Int32Array(rows);
  let day = Date.UTC(2026, 9, 30) / 86_400_000;
  for (let k = rows - 1; k >= 0; day--) {
    const weekday = (((day + 4) % 7) + 7) % 7;
    if (weekday !== 0 && weekday !== 6) days[k--] = day;
  }
  const ids = [];
  const starts = new Int32Array(securities);
  const dates = new Int32Array(securities * rows);
  const close = new Float64Array(securities * rows);
  const volume = new Float64Array(securities * rows);
  const dividends = new Float64Array(securities * rows);
  let n = 0;
  for (let i = 0; i < securities; i++) {
    ids.push(i === 0 ? "INDEX" : `S${String(i).padStart(5, "0")}`);
    starts[i] = n;
    const first = i === 0 ? 0 : Math.floor(uniform() * (rows / 10));
    const drift = 0.0002 + 0.0004 * uniform();
    const scale = 0.008 + 0.02 * uniform();
    let price = 20 + 180 * uniform();
    for (let k = first; k < rows; k++, n++) {
      price *= Math.exp(drift + scale * normal());
      dates[n] = days[k];
      close[n] = price;
      volume[n] = 1e5 + 1e7 * uniform();
      dividends[n] = k % 63 === 0 ? 0.005 * price : 0;
    }
  }
  return {
    ids,
    starts,
    dates: dates.subarray(0, n),
    close: close.subarray(0, n),
    volume: volume.subarray(0, n),
    dividends: dividends.subarray(0, n),
  };
}

const columns = largeUniverse(5000, 10);
let started = performance.now();
const universe = fl.Universe.fromColumns(columns, { benchmark: "INDEX", risk_free_rate: 0.03 });
const built = (performance.now() - started) / 1000;
let matched = 0;
const screened = median(() => {
  matched = universe.screen().results.coverage.matched;
});
console.log(
  `universe of ${universe.length} securities and ${columns.close.length} closes from typed arrays in WebAssembly: ${built.toFixed(3)} s`,
);
report(`screen of ${universe.length} securities by 10 years of daily closes in WebAssembly`, screened, 1);
if (matched === 0) {
  console.log("the screen kept no security");
  failed = true;
}
process.exit(failed ? 1 : 0);
