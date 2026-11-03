// Comparing methods and backtesting one allocation, as the package's entry for Node.js gives them,
// and the reports' own methods for laying out and showing a report.
import assert from "node:assert/strict";
import { readFile, rm } from "node:fs/promises";
import { test } from "node:test";

import { Assets, datasets } from "../pkg/node.js";

const assets = new Assets(datasets.assetClassPrices(), { risk_free_asset: "T-Bills" });

function paths(run) {
  try {
    run();
  } catch (e) {
    assert.equal(e.name, "ValidationError");
    return e.issues.map((issue) => issue.path);
  }
  assert.fail("no error");
}

test("methods compared walk-forward with the same limits and costs", () => {
  const backtest = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: "quarterly",
    validation: { walk_forward: { years: 5 } },
    trading_cost_bps: 10,
  });
  assert.deepEqual(
    backtest.strategies.map((s) => s.name),
    ["Equal weight", "Minimum variance", "Risk parity"],
  );
  assert.equal(backtest.settings.validation.type, "walk_forward");
  assert.equal(backtest.settings.trading_cost_bps, 10);
  const minimum = backtest.strategies[1];
  assert.equal(minimum.out_of_sample.dates.length, assets.dates.length - 60);
  assert.ok(Object.values(minimum.out_of_sample.trades.at(-1).weights).every((w) => w <= 0.4 + 1e-6));
  assert.equal(backtest.strategies[2].notes.length, 1, "risk parity takes no limits");
  // Tagged data gives the same result as the shortcuts.
  const tagged = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: { type: "quarterly" },
    validation: { type: "walk_forward", years: 5 },
    trading_cost_bps: 10,
  });
  assert.deepEqual(tagged, backtest);
});

test("one method, or fixed weights, against a benchmark", () => {
  const backtest = assets.backtest({
    method: "maximum_sharpe",
    max_weight: 0.4,
    benchmark: { "US Equity": 0.6, Bonds: 0.4 },
    validation: { holdout: { start: "2016-11-01" } },
  });
  const [strategy] = backtest.strategies;
  assert.equal(strategy.name, "Maximum Sharpe");
  assert.equal(strategy.out_of_sample.dates[0], "2016-11-30");
  assert.equal(backtest.benchmark.name, "Benchmark");
  const fixed = assets.backtest({
    weights: { "US Equity": 0.6, Bonds: 0.4 },
    name: "60/40",
    rebalance: { drift: { band: 0.05 } },
  });
  assert.equal(fixed.strategies[0].name, "60/40");
  assert.deepEqual(fixed.settings.rebalance, { type: "drift", band: 0.05 });
});

test("an index as the benchmark", async () => {
  const sp500 = new Assets(await datasets.sp500(), { risk_free_asset: "T-Bills" });
  const backtest = sp500.backtest({
    method: "equal_weight",
    rebalance: "yearly",
    benchmark: await datasets.sp500Index(),
  });
  assert.equal(backtest.benchmark.name, "S&P 500");
  assert.equal(backtest.benchmark.in_sample.returns.length, sp500.dates.length);
});

test("mistakes are reported where they were made", () => {
  assert.deepEqual(paths(() => assets.compare({ methods: ["minimum_variance"], objective: "x" })), [
    "options.objective",
  ]);
  assert.deepEqual(paths(() => assets.compare({ max_weight: 0.4 })), ["options.methods"]);
  assert.deepEqual(paths(() => assets.compare({ methods: ["maximum_profit"] })).length, 1);
  assert.deepEqual(paths(() => assets.compare({ methods: ["minimum_variance"], max_weight: "x" })), [
    "options.max_weight",
  ]);
  assert.deepEqual(paths(() => assets.backtest({ weights: { Bonds: 1 }, max_weight: 0.4 })), [
    "options.max_weight",
  ]);
  assert.deepEqual(paths(() => assets.backtest({ name: "none" })), ["options.method"]);
  assert.deepEqual(
    paths(() => assets.backtest({ method: "equal_weight", validation: { holdout: { start: "2099-01-01" } } })),
    ["validation.start"],
  );
  assert.deepEqual(paths(() => assets.optimise({ objective: "minimum_variance" })), ["options.objective"]);
});

test("a report lays itself out as text and HTML, and shows", async () => {
  const portfolio = datasets.portfolio();
  const report = portfolio.report({ ...datasets.reportOptions(), name: "Morgan family portfolio" });
  assert.equal(report.name, "Morgan family portfolio");
  assert.ok(report.toHtml().includes("<title>Morgan family portfolio</title>"));
  assert.ok(report.toHtml("Another").includes("<title>Another</title>"));
  assert.equal(report.toText().split("Morgan family portfolio").length - 1, 6);
  // The methods are not data: the report still writes to JSON and clones as before.
  assert.deepEqual(Object.keys(report), Object.keys(JSON.parse(JSON.stringify(report))));
  assert.equal(structuredClone(report).toHtml, undefined);
  const before = process.env.BROWSER;
  process.env.BROWSER = "true";
  try {
    const path = report.show();
    assert.ok((await readFile(path, "utf8")).startsWith("<!doctype html>"));
    await rm(path);
  } finally {
    if (before === undefined) delete process.env.BROWSER;
    else process.env.BROWSER = before;
  }
  assert.deepEqual(paths(() => portfolio.report({ benchmrk: "BENCH" })), ["options.benchmrk"]);
});
