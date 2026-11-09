import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine } from "./setup.mjs";

const fl = await engine();

test("assets from the asset-class dataset", () => {
  const prices = fl.datasets.assetClassPrices();
  assert.deepEqual(prices.labels, ["US Equity", "Intl Equity", "Bonds", "T-Bills", "Gold", "Real Estate"]);
  const assets = new fl.Assets(prices, { risk_free_asset: "T-Bills" });
  assert.equal(assets.periodsPerYear, 12);
  assert.equal(assets.dates.length, prices.dates.length - 1);
  assert.equal(assets.riskFreeAsset, "T-Bills");
  close(assets.riskFreeRate, assets.meanReturns()["T-Bills"], 1e-15, "risk-free rate");
  const result = assets.optimise({
    method: "maximum_sharpe",
    bounds: { min: 0, max: 0.4 },
    groups: { Equity: { assets: ["US Equity", "Intl Equity"], min: 0.3, max: 0.6 } },
  });
  const weights = Object.values(result.results.weights);
  close(weights.reduce((a, b) => a + b, 0), 1, 1e-6, "weights");
  assert.deepEqual(Object.keys(result.results.weights), ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]);
  const frontier = assets.frontier(5, { bounds: { min: 0, max: 0.5 } });
  assert.equal(frontier.length, 5);
  assert.deepEqual(Object.keys(frontier[0].weights), Object.keys(result.results.weights));
  assert.deepEqual(Object.keys(assets.meanReturns()), assets.labels);
  assert.throws(() => assets.optimise({ method: "target_return" }), (e) => {
    assert.equal(e.name, "ValidationError");
    assert.deepEqual(e.issues.map((i) => i.path), ["options.target"]);
    return true;
  });
  const fromCsv = fl.Assets.fromCsv(fl.datasets.file("asset-classes.csv"));
  assert.deepEqual(fromCsv.labels, assets.labels);
  assert.deepEqual(fromCsv.returns[0], assets.returns[0]);
});

test("limits on cash let optimisation hold it", () => {
  const assets = new fl.Assets(fl.datasets.assetClassPrices(), { risk_free_asset: "T-Bills" });
  const result = assets.optimise({ method: "minimum_variance", cash: { min: 0, max: 0.3 } });
  close(result.results.weights["T-Bills"], 0.3, 1e-6, "cash");
  assert.deepEqual(result.report.context.limits, ["0% to 30% in cash"]);
  // Without a risk-free asset, the cash earns the risk-free rate.
  const plain = new fl.Assets(fl.datasets.assetClassPrices(), { risk_free_rate: 0.02 });
  const held = plain.optimise({ method: "minimum_variance", cash: { min: 0.1, max: 0.2 } });
  close(held.results.weights.Cash, 0.2, 1e-6, "Cash");
  assert.throws(() => assets.optimise({ method: "maximum_sharpe", cash: { min: 0, max: 0.2 } }), (e) => {
    assert.equal(e.issues[0].path, "options.cash");
    return true;
  });
});

test("given weights are valued as optimisation values its own", () => {
  const portfolio = fl.datasets.portfolio();
  const valued = portfolio.assets().evaluate({ weights: portfolio.weights() });
  const current = portfolio.report().overview.current;
  close(valued.results.volatility, current.volatility, 1e-12, "volatility");
  close(valued.results.expected_return, current.expected_return, 1e-12, "expected return");
  assert.ok(valued.report.context.given);
  assert.deepEqual(Object.keys(valued.results.weights).slice(0, 5), ["BND", "EXSA", "GLD", "VNQ", "VTI"]);
  const assets = new fl.Assets(fl.datasets.assetClassPrices(), { risk_free_asset: "T-Bills" });
  const framed = assets.evaluate({
    weights: { "US Equity": 0.6, Bonds: 0.4 },
    max_weight: 0.5,
    frontier: true,
    name: "Sixty forty",
  });
  assert.equal(framed.report.frontier.length, 30);
  assert.equal(framed.report.name, "Sixty forty");
  assert.match(fl.resultsText(framed.results), /Evaluation · Given weights/);
  assert.throws(() => assets.evaluate({ weights: { "US Equity": 0.6 }, max_weight: 0.5 }), (e) => {
    assert.deepEqual(e.issues.map((i) => i.path), ["options.max_weight", "options.weights"]);
    return true;
  });
});

test("the portfolio dataset, from report to trades", () => {
  const portfolio = fl.datasets.portfolio();
  assert.equal(portfolio.asOf, "2026-10-30");
  assert.equal(portfolio.data.transactions.length, 203);
  const report = portfolio.report(fl.datasets.reportOptions());
  assert.equal(report.scenarios.scenarios.length, 4);
  assert.ok(report.attribution);
  const assets = portfolio.assets();
  assert.deepEqual(assets.labels, ["BND", "EXSA", "GLD", "VNQ", "VTI", "EUR cash", "USD cash"]);
  // Optimisation chooses among the instruments; the cash is there to value the portfolio.
  assert.deepEqual(assets.riskyAssets, ["BND", "EXSA", "GLD", "VNQ", "VTI"]);
  assert.deepEqual(assets.cashAssets, ["USD cash", "EUR cash"]);
  assert.equal(assets.riskFreeAsset, "USD cash");
  const result = assets.optimise({ method: "minimum_variance", bounds: { min: 0.05, max: 0.5 } });
  const proposal = portfolio.rebalance(result);
  assert.deepEqual(proposal.results, portfolio.rebalance(result.results).results);
  assert.deepEqual(proposal.results, portfolio.rebalance(result.results.weights).results);
  assert.equal(proposal.results.accounts.length, 2);
  const after = portfolio.withRebalance(proposal).weights();
  assert.deepEqual(after, portfolio.withRebalance(proposal.results).weights());
  for (const [holding, target] of Object.entries(result.results.weights)) {
    close(after[holding], target, 0.01, holding);
  }
  const failed = assets.optimise({ method: "target_return", target: 0.9 });
  assert.equal(failed.results.weights, null);
  for (const targets of [failed, failed.results]) {
    assert.throws(() => portfolio.rebalance(targets), (e) => {
      assert.equal(e.name, "ValidationError");
      assert.deepEqual(
        e.issues.map((i) => [i.path, i.message]),
        [["targets", "the optimisation found no weights: no portfolio meets every limit"]],
      );
      return true;
    });
  }
});

test("fees are an expense, not a loss on cash", () => {
  const performance = fl.datasets.portfolio().performance();
  const contributions = performance.contributions();
  const expenses = performance.expenses();
  assert.ok(expenses < 0);
  const total = contributions.reduce((sum, c) => sum + c.contribution, 0) + expenses;
  close(total, performance.timeWeighted(), 1e-12, "contributions and expenses");
  const usd = contributions.find((c) => c.holding === "USD cash");
  assert.ok(Math.abs(usd.return_base) < 1e-12);
});

test("reports print as text and lay out as one HTML page", () => {
  const portfolio = fl.datasets.portfolio();
  const report = portfolio.report(fl.datasets.reportOptions());
  const text = fl.reportText(report, "Morgan family portfolio");
  assert.ok(text.split("\n").every((line) => [...line].length <= 80));
  assert.equal(text.split("Morgan family portfolio").length - 1, 6);
  assert.ok(fl.reportText(report.risk).includes("FINALYTICS  ·  RISK"));
  const html = fl.reportHtml(report, "Morgan family portfolio");
  assert.ok(html.startsWith("<!doctype html>"));
  assert.ok(html.includes("<title>Morgan family portfolio</title>"));
  assert.ok(!html.includes("<script"));
});

test("a portfolio from its transactions alone", () => {
  const portfolio = fl.Portfolio.fromCsv({
    transactions: "date,type,instrument,quantity,price,amount,currency\n2026-01-02,deposit,,,,20000,USD\n" +
      "2026-01-02,buy,VOO,20,500,,\n2026-01-02,buy,AGG,100,98,,\n",
    prices: "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n",
  });
  assert.equal(portfolio.baseCurrency, "USD");
  assert.deepEqual(portfolio.data.accounts.map((a) => a.id), ["main"]);
  assert.equal(portfolio.valuation().total_base, "20120.0");
  assert.deepEqual(fl.Portfolio.fromJson(portfolio.toJson()).valuation(), portfolio.valuation());
  assert.throws(() => fl.Portfolio.fromCsv({ transactions: "date,type\n2026-01-02,buy\n", extra: "" }), {
    name: "ValidationError",
  });
});

test("rebalancing keeps the cash asked", () => {
  const portfolio = fl.datasets.portfolio();
  const target = portfolio.assets().optimise({ method: "minimum_variance", bounds: { min: 0.05, max: 0.4 } });
  const kept = portfolio.rebalance(target, { keep_cash: ["USD"] });
  const r = kept.results;
  assert.ok(Math.abs(r.weights_after["USD cash"] - r.weights_before["USD cash"]) * Number(r.value_base) < 0.01);
  assert.deepEqual(r.kept_cash.map((k) => k.currency), ["USD"]);
  assert.equal(portfolio.rebalance(target, { keep_cash: true }).results.kept_cash.length, 2);
});
