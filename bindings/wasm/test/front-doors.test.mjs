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
    objective: "maximum_sharpe",
    bounds: { min: 0, max: 0.4 },
    groups: { Equity: { assets: ["US Equity", "Intl Equity"], min: 0.3, max: 0.6 } },
  });
  const weights = Object.values(result.weights);
  close(weights.reduce((a, b) => a + b, 0), 1, 1e-6, "weights");
  assert.deepEqual(Object.keys(result.weights), ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]);
  assert.equal(assets.frontier(5, { bounds: { min: 0, max: 0.5 } }).length, 5);
  assert.throws(() => assets.optimise({ objective: "target_return" }), (e) => {
    assert.equal(e.name, "ValidationError");
    assert.deepEqual(e.issues.map((i) => i.path), ["options.target"]);
    return true;
  });
  const fromCsv = fl.Assets.fromCsv(fl.datasets.file("asset-classes.csv"));
  assert.deepEqual(fromCsv.labels, assets.labels);
  assert.deepEqual(fromCsv.returns[0], assets.returns[0]);
});

test("the portfolio dataset, from report to trades", () => {
  const portfolio = fl.datasets.portfolio();
  assert.equal(portfolio.asOf, "2026-10-30");
  assert.equal(portfolio.data.transactions.length, 148);
  const report = portfolio.report(fl.datasets.reportOptions());
  assert.equal(report.scenarios.scenarios.length, 4);
  assert.ok(report.attribution);
  const assets = portfolio.assets();
  assert.deepEqual(assets.labels, ["BOND", "GOLD", "INTL", "REIT", "USEQ"]);
  const targets = assets.optimise({ objective: "minimum_variance", bounds: { min: 0.05, max: 0.5 } }).weights;
  const proposal = portfolio.rebalance(targets);
  assert.equal(proposal.accounts.length, 2);
  const after = portfolio.withRebalance(proposal).weights();
  for (const [holding, target] of Object.entries(targets)) {
    close(after[holding], target, 0.01, holding);
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
