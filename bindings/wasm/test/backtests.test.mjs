// Comparing methods and backtesting one allocation, as the package's entry for Node.js gives them:
// the optimisation and backtest reports, and the reports' own methods for laying out and showing
// a report.
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
  const report = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: "quarterly",
    validation: { walk_forward: { years: 5 } },
    trading_cost_bps: 10,
    name: "Six asset classes",
  });
  assert.equal(report.name, "Six asset classes");
  assert.deepEqual(
    report.strategies.map((s) => s.name),
    ["Equal weight", "Minimum variance", "Risk parity"],
  );
  assert.equal(report.context.settings.validation.type, "walk_forward");
  assert.equal(report.context.settings.trading_cost_bps, 10);
  assert.equal(report.context.window, 60);
  const minimum = report.strategies[1];
  assert.equal(minimum.tested.growth.dates.length, assets.dates.length - 60);
  assert.ok(Object.values(minimum.weights.at(-1).weights).every((w) => w <= 0.4 + 1e-6));
  assert.equal(report.strategies[2].notes.length, 1, "risk parity takes no limits");
  assert.ok(report.toText().includes("FINALYTICS  ·  OPTIMISATION"));
  assert.ok(report.toHtml().includes("<title>Six asset classes</title>"));
  // Tagged data gives the same result as the shortcuts.
  const tagged = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: { type: "quarterly" },
    validation: { type: "walk_forward", years: 5 },
    trading_cost_bps: 10,
    name: "Six asset classes",
  });
  assert.deepEqual(tagged, report);
});

test("one method, or fixed weights, against a benchmark", () => {
  const report = assets.backtest({
    method: "maximum_sharpe",
    max_weight: 0.4,
    benchmark: { "US Equity": 0.6, Bonds: 0.4 },
    validation: { holdout: { start: "2016-11-01" } },
  });
  assert.equal(report.strategy.name, "Maximum Sharpe");
  assert.equal(report.strategy.tested.start, "2016-11-30");
  assert.equal(report.history.split, "2016-11-30");
  assert.equal(report.benchmark.name, "Benchmark");
  assert.ok(report.toHtml().includes("Backtest report"));
  const fixed = assets.backtest({
    weights: { "US Equity": 0.6, Bonds: 0.4 },
    name: "60/40",
    rebalance: { drift: { band: 0.05 } },
  });
  assert.equal(fixed.strategy.name, "60/40");
  assert.deepEqual(fixed.context.settings.rebalance, { type: "drift", band: 0.05 });
  // The results alone, without a report.
  const raw = assets.simulate({ strategies: [{ type: "weights", weights: { Bonds: 1 } }] });
  assert.equal(raw.strategies[0].in_sample.returns.length, assets.dates.length);
});

test("an index as the benchmark", async () => {
  const sp500 = new Assets(await datasets.sp500(), { risk_free_asset: "T-Bills" });
  const report = sp500.backtest({
    method: "equal_weight",
    rebalance: "yearly",
    benchmark: await datasets.sp500Index(),
  });
  assert.equal(report.benchmark.name, "S&P 500");
  assert.equal(report.benchmark.tested.statistics.observations, sp500.dates.length);
  assert.ok(report.strategy.tested.growth.dates.length < 300, "daily returns are charted by month");
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
    const compared = assets.compare({ methods: ["equal_weight", "minimum_variance"] }).show();
    assert.ok((await readFile(compared, "utf8")).includes("Optimisation report"));
    await rm(compared);
  } finally {
    if (before === undefined) delete process.env.BROWSER;
    else process.env.BROWSER = before;
  }
  assert.deepEqual(paths(() => portfolio.report({ benchmrk: "BENCH" })), ["options.benchmrk"]);
});
