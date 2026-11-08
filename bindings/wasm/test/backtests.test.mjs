// Optimising, comparing methods and backtesting one allocation, as the package's entry for Node.js
// gives them: each result has a report and the raw results, prints as text and shows as a page.
import assert from "node:assert/strict";
import { readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { Assets, datasets } from "../pkg/node.js";
import { close, priceTable, text } from "./setup.mjs";

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

test("every result has a report and the raw results", () => {
  const results = [
    [assets.optimise({ method: "maximum_sharpe", max_weight: 0.4 }), "OPTIMISATION", "OptimisationResult"],
    [assets.compare({ methods: ["equal_weight", "minimum_variance"] }), "COMPARISON", "BacktestResults"],
    [assets.backtest({ method: "minimum_variance" }), "BACKTEST", "BacktestResults"],
  ];
  for (const [result, title, raw] of results) {
    assert.deepEqual(Object.keys(result), ["report", "results"]);
    const text = `${result}`;
    assert.equal(text, result.toText());
    assert.equal(text, result.report.toText());
    assert.ok(text.includes(title));
    assert.ok(text.split("\n").every((line) => [...line].length <= 80));
    assert.equal(result.toHtml(), result.report.toHtml());
    assert.equal(result.toText("Named"), result.report.toText("Named"));
    const printed = `${result.results}`;
    assert.ok(printed.startsWith(raw), printed);
    assert.ok(printed.split("\n").every((line) => [...line].length <= 80));
  }
});

test("the optimisation report explains the weights", async () => {
  const result = assets.optimise({ method: "maximum_sharpe", max_weight: 0.4 });
  const { report, results } = result;
  close(report.summary.volatility, results.volatility, 1e-12, "volatility");
  assert.equal(report.allocation[0].asset, report.summary.largest_asset);
  close(report.allocation.reduce((sum, a) => sum + a.risk_share, 0), 1, 1e-9, "risk shares");
  assert.deepEqual(report.context.limits, ["At most 40% in any asset"]);
  assert.equal(report.history.weights.statistics.observations, assets.dates.length);
  assert.deepEqual(Object.keys(results.weights), ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]);
  assert.ok(result.toHtml("Six asset classes").includes("<title>Six asset classes</title>"));
  const named = assets.optimise({ method: "maximum_sharpe", max_weight: 0.4, name: "Six asset classes" });
  assert.equal(named.report.name, "Six asset classes");
  // A page saved to a file, as in Python and Rust.
  const file = join(tmpdir(), `finalytics-saved-${process.pid}.html`);
  assert.equal(result.saveHtml(file, "Saved"), file);
  assert.ok((await readFile(file, "utf8")).includes("<title>Saved</title>"));
  // In a theme of its own, whatever the reader's setting.
  result.saveHtml(file, undefined, "dark");
  assert.ok((await readFile(file, "utf8")).includes('<html lang="en" data-theme="dark">'));
  await rm(file);
  assert.ok(result.toHtml(undefined, "light").includes('data-theme="light"'));
  assert.ok(result.toHtml().includes('<html lang="en">'));
  assert.throws(() => result.toHtml(undefined, "night"), (e) => e.name === "ValidationError");
  // The frontier only when asked for: it changes the report, not the weights.
  assert.deepEqual(report.frontier, []);
  assert.ok(result.toHtml().includes("frontier: true in JavaScript"));
  const drawn = assets.optimise({ method: "maximum_sharpe", max_weight: 0.4, frontier: true });
  assert.ok(drawn.report.frontier.length > 10);
  assert.deepEqual(drawn.results.weights, results.weights);
  assert.ok(drawn.toHtml().includes("The efficient frontier"));
  assert.deepEqual(paths(() => assets.compare({ methods: ["equal_weight"], frontier: true })), [
    "options.frontier",
  ]);
});

test("methods compared walk-forward with the same limits and costs", () => {
  const result = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: "quarterly",
    validation: { walk_forward: { years: 5 } },
    trading_cost_bps: 10,
    name: "Six asset classes",
  });
  const { report } = result;
  assert.equal(report.name, "Six asset classes");
  assert.deepEqual(
    report.strategies.map((s) => s.name),
    ["Equal weight", "Minimum variance", "Risk parity"],
  );
  assert.equal(report.context.settings.validation.type, "walk_forward");
  assert.equal(report.context.settings.trading_cost_bps, 10);
  assert.equal(report.context.window, 60);
  const minimum = report.strategies[1];
  // From 100 at the end of the period before the test.
  assert.equal(minimum.tested.growth.dates.length, assets.dates.length - 60 + 1);
  assert.equal(minimum.tested.growth.values[0], 100);
  assert.ok(Object.values(minimum.weights.at(-1).weights).every((w) => w <= 0.4 + 1e-6));
  assert.equal(report.strategies[2].notes.length, 1, "risk parity takes no limits");
  assert.ok(result.toText().includes("FINALYTICS  ·  COMPARISON"));
  assert.ok(result.toHtml().includes("<title>Six asset classes</title>"));
  assert.equal(result.results.strategies[1].out_of_sample.returns.length, assets.dates.length - 60);
  // Tagged data gives the same result as the shortcuts.
  const tagged = assets.compare({
    methods: ["equal_weight", "minimum_variance", "risk_parity"],
    max_weight: 0.4,
    rebalance: { type: "quarterly" },
    validation: { type: "walk_forward", years: 5 },
    trading_cost_bps: 10,
    name: "Six asset classes",
  });
  assert.deepEqual(tagged, result);
});

test("one method, or fixed weights, against a benchmark", () => {
  const result = assets.backtest({
    method: "maximum_sharpe",
    max_weight: 0.4,
    benchmark: { "US Equity": 0.6, Bonds: 0.4 },
    validation: { holdout: { start: "2016-11-01" } },
  });
  const { report } = result;
  assert.equal(report.strategy.name, "Maximum Sharpe");
  assert.equal(report.strategy.tested.start, "2016-11-30");
  assert.equal(report.history.split, "2016-11-30");
  assert.equal(report.benchmark.name, "Benchmark");
  assert.ok(result.toHtml().includes("Backtest report"));
  const fixed = assets.backtest({
    weights: { "US Equity": 0.6, Bonds: 0.4 },
    name: "60/40",
    rebalance: { drift: { band: 0.05 } },
  });
  assert.equal(fixed.report.strategy.name, "60/40");
  assert.deepEqual(fixed.report.context.settings.rebalance, { type: "drift", band: 0.05 });
  // Every return and trade behind the report.
  assert.equal(fixed.results.strategies[0].in_sample.returns.length, assets.dates.length);
  // The growth has a point more: 100 on the date the test enters.
  assert.equal(
    result.results.strategies[0].out_of_sample.returns.length,
    report.strategy.tested.growth.dates.length - 1,
  );
});

test("an index as the benchmark", async () => {
  const sp500 = Assets.fromCsv(await text("tests/sp500/prices.csv"), { risk_free_asset: "T-Bills" });
  const { report } = sp500.backtest({
    method: "equal_weight",
    rebalance: "yearly",
    benchmark: priceTable(await text("tests/sp500/index.csv")),
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
  assert.deepEqual(paths(() => assets.frontier(5, { max_wieght: 0.4 })), ["options.max_wieght"]);
  // Numbers JSON cannot hold are reported, not dropped.
  assert.deepEqual(paths(() => assets.optimise({ method: "minimum_variance", max_weight: NaN })), [
    "options.max_weight",
  ]);
  assert.deepEqual(paths(() => assets.compare({ methods: ["minimum_cvar"], confidence: Infinity })), [
    "options.confidence",
  ]);
  // Every problem with the limits, before solving.
  assert.deepEqual(paths(() => assets.optimise({ method: "minimum_variance", max_weight: 0.1 })), [
    "options.max_weight",
  ]);
  assert.deepEqual(paths(() => assets.optimise({ method: "maximum_sharpe", confidence: 0.9 })), [
    "options.confidence",
  ]);
});

test("a rebalance has a report and the raw results", () => {
  const portfolio = datasets.portfolio();
  const target = portfolio.assets().optimise({ method: "minimum_variance", bounds: { min: 0.05, max: 0.4 } });
  const result = portfolio.rebalance(target, {
    whole_units: true,
    fee_bps: 5,
    minimum_fee: 1,
    tax: [0.37, 0.2],
    name: "Morgan family portfolio",
  });
  assert.deepEqual(Object.keys(result), ["report", "results"]);
  const text = `${result}`;
  assert.equal(text, result.report.toText());
  assert.ok(text.includes("FINALYTICS  ·  REBALANCE") && text.includes("Morgan family portfolio"));
  assert.ok(text.split("\n").every((line) => [...line].length <= 80));
  assert.ok(`${result.results}`.startsWith("RebalanceResults · 2 accounts"));
  assert.ok(result.toHtml().includes("Rebalance report"));
  const [brokerage, ira] = result.report.accounts;
  assert.ok(brokerage.taxable && brokerage.tax_base > 0);
  assert.ok(!ira.taxable && ira.tax_base == null && ira.realised_gain_base > 0);
  assert.ok(result.report.accounts.every((a) => a.warnings.length === 0));
  assert.equal(result.report.context.method, "Minimum variance");
  assert.deepEqual(result.report.context.fees, { rate: "0.0005", per_trade: "0", minimum: "1" });
  assert.deepEqual(result.report.context.tax, { short_term: "0.37", long_term: "0.2", long_term_after_days: 365 });
  assert.deepEqual(paths(() => portfolio.rebalance(target, { fees: { rate: "0.001" } })), ["choices.fees"]);
  const after = portfolio.withRebalance(result);
  assert.ok(Number(after.valuation().total_base) > 0);
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
    assert.ok((await readFile(compared, "utf8")).includes("Comparison report"));
    await rm(compared);
    const optimised = assets.optimise({ method: "minimum_variance" }).show();
    assert.ok((await readFile(optimised, "utf8")).includes("Optimisation report"));
    await rm(optimised);
  } finally {
    if (before === undefined) delete process.env.BROWSER;
    else process.env.BROWSER = before;
  }
  assert.deepEqual(paths(() => portfolio.report({ benchmrk: "BENCH" })), ["options.benchmrk"]);
});

test("fixed weights are compared with methods on the same periods", () => {
  const result = assets.compare({
    methods: ["minimum_variance"],
    weights: { "60/40": { "US Equity": 0.6, Bonds: 0.4 } },
    validation: { walk_forward: { years: 5 } },
    benchmark: { "US Equity": 1 },
    benchmark_name: "Stocks",
  });
  const { strategies, benchmark } = result.report;
  assert.deepEqual(
    strategies.map((s) => s.name),
    ["Minimum variance", "60/40"],
  );
  assert.equal(strategies[1].fitted, false);
  assert.equal(strategies[1].tested.start, strategies[0].tested.start);
  assert.equal(benchmark.name, "Stocks");
  assert.deepEqual(paths(() => assets.compare({ benchmark_name: "x", methods: ["equal_weight"] })), [
    "options.benchmark_name",
  ]);
  assert.deepEqual(paths(() => assets.compare({})), ["options.methods"]);
});

test("fixed weights are not called in sample, and keep their names", () => {
  const result = assets.compare({
    weights: { "Min-CVaR": { "US Equity": 0.3, Bonds: 0.7 }, Growth: { "US Equity": 0.8, Gold: 0.2 } },
    benchmark: { "US Equity": 0.6, Bonds: 0.4 },
    benchmark_name: "60/40",
  });
  const text = result.report.toText();
  const page = result.report.toHtml();
  for (const shown of [text, page]) {
    assert.ok(!shown.toLowerCase().includes("in sample"));
    assert.ok(shown.includes("2 portfolios"));
  }
  assert.ok(page.includes("60/40, the benchmark, is not ranked"));
  assert.ok(!page.includes("min-CVaR"));
});

test("the downside methods report their optimum", () => {
  const semi = assets.optimise({ method: "minimum_semivariance" });
  close(semi.report.summary.downside_deviation, Math.sqrt(semi.results.solution.objective_value), 1e-15, "deviation");
  assert.match(semi.report.toText(), /Downside deviation/);
  const cdar = assets.optimise({ method: "minimum_cdar", confidence: 0.9 });
  const figure = cdar.report.summary.drawdown_at_risk;
  assert.equal(figure.confidence, 0.9);
  assert.equal(figure.value, cdar.results.solution.objective_value);
});

test("a backtest follows money in and out", () => {
  const result = assets.backtest({
    weights: { "US Equity": 0.6, Bonds: 0.4 },
    start_value: 10000,
    flows: [
      { date: "2020-01-31", amount: 1000 },
      { date: "2021-06-15", amount: -500 },
    ],
  });
  const money = result.report.strategy.tested.money;
  assert.equal(money.start_value, 10000);
  assert.equal(money.paid_in, 1000);
  assert.equal(money.taken_out, 500);
  assert.equal(money.flows, 2);
  assert.ok(`${result}`.includes("Money in and out"));
});

test("a comparison keeps the strategies in the order given", () => {
  const names = ["Zero bonds", "Minimum variance", "Equal weight", "All bonds", "A gold tilt"];
  const result = assets.compare({
    methods: [{ name: "Zero bonds", weights: { "US Equity": 0.7, Gold: 0.3 } }, "minimum_variance", "equal_weight"],
    weights: { "All bonds": { Bonds: 1 }, "A gold tilt": { "US Equity": 0.5, Gold: 0.5 } },
  });
  assert.deepEqual(result.results.strategies.map((s) => s.name), names);
  assert.deepEqual(result.report.strategies.map((s) => s.name), names);
  const at = names.map((n) => `${result}`.indexOf(n));
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
  assert.throws(
    () => assets.compare({ methods: [{ weights: { Bonds: 1 } }] }),
    (e) => e.name === "ValidationError" && e.issues[0].path === "options.methods[0].name",
  );
});
