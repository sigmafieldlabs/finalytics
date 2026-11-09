// The contracts between features: each verb takes what the others give it, in every shape of
// data: the sample household's assets (five funds, euro cash, and dollar cash as the risk-free
// asset), the asset classes with T-bills as the risk-free asset, and the asset classes without
// them at a 2% rate.
import assert from "node:assert/strict";
import { test } from "node:test";

import { engine } from "./setup.mjs";

const fl = await engine();

const portfolio = fl.datasets.portfolio();
const household = portfolio.assets();
const prices = fl.datasets.assetClassPrices();
const bills = new fl.Assets(prices, { risk_free_asset: "T-Bills" });
const keep = prices.labels.map((l, i) => [l, i]).filter(([l]) => l !== "T-Bills");
const rate = new fl.Assets(
  { dates: prices.dates, labels: keep.map(([l]) => l), columns: keep.map(([, i]) => prices.columns[i]) },
  { risk_free_rate: 0.02 },
);
const shapes = { household, bills, rate };

test("minimum CVaR with cash reports the shortfall of every weight", () => {
  for (const [shape, assets] of Object.entries(shapes)) {
    const result = assets.optimise({ method: "minimum_cvar", cash: { min: 0, max: 0.2 } });
    const cash = assets.riskFreeAsset ?? "Cash";
    assert.ok(cash in result.results.weights, shape);
    assert.ok(result.report.summary.expected_shortfall, shape);
    const shares = result.report.allocation.reduce((s, a) => s + a.risk_share, 0);
    assert.ok(Math.abs(shares - 1) < 1e-6, `${shape}: ${shares}`);
    assert.ok(result.report.allocation.some((a) => a.asset === cash), shape);
  }
});

const walkForward = (assets) => ({ walk_forward: { years: assets.periodsPerYear > 100 ? 1 : 5 } });

test("the engine's own weights pass its own checks", () => {
  const cases = [
    { method: "minimum_cdar" },
    { method: "mean_variance", risk_aversion: 3, max_weight: 0.35 },
    { method: "minimum_variance", cash: { min: 0, max: 0 } },
  ];
  for (const options of cases) {
    const result = household.optimise(options);
    const weights = result.results.weights;
    const values = Object.values(weights);
    assert.ok(values.every((w) => w >= 0), JSON.stringify(weights));
    assert.ok(values.every((w) => w === 0 || w >= 1e-7), JSON.stringify(weights));
    assert.ok(Math.abs(values.reduce((s, w) => s + w, 0) - 1) < 1e-12);
    const valued = household.evaluate({ weights });
    assert.ok(Math.abs(valued.results.volatility - result.results.volatility) < 1e-12);
    portfolio.rebalance(result);
    household.optimise({ method: "minimum_variance", current_weights: weights });
  }
  const capped = bills.optimise({ method: "maximum_sharpe", max_weight: 0.3 });
  assert.ok(Object.values(capped.results.weights).every((w) => w <= 0.3));
});

test("a walk-forward refits from the weights it holds without checking them as given", () => {
  for (const [shape, assets] of Object.entries(shapes)) {
    const validation = walkForward(assets);
    assert.doesNotThrow(() => assets.backtest({ method: "mean_variance", risk_aversion: 3, min_trade: 0.01, validation }), shape);
    assert.doesNotThrow(() => assets.backtest({ method: "minimum_cdar", max_turnover: 0.5, validation }), shape);
  }
});

test("a refit after fits that held only cash enters from cash", () => {
  const column = bills.labels.indexOf("US Equity");
  const one = fl.Assets.fromReturns(bills.dates, ["US Equity"], [bills.returns[column]], { risk_free_rate: 0.02 });
  const target = one.meanReturns()["US Equity"];
  const backtest = one.backtest({ method: "target_return", target, max_turnover: 0.5, validation: { walk_forward: { years: 5 } } });
  const strategy = backtest.results.strategies[0];
  assert.ok(strategy.failures.length > 0);
  assert.ok(strategy.out_of_sample.trades.some((t) => t.refit && t.turnover > 0.5 + 1e-9));
});

/** The household's own weights, its euro cash counted with the dollar cash. */
function folded() {
  const out = {};
  for (const [key, w] of Object.entries(portfolio.weights())) {
    const to = household.riskyAssets.includes(key) ? key : household.riskFreeAsset;
    out[to] = (out[to] ?? 0) + w;
  }
  return out;
}

test("rebalancing to an optimisation trades what it traded and nothing else", () => {
  const optimisation = household.optimise({
    method: "minimum_variance",
    current_weights: folded(),
    max_turnover: 0.1,
    max_trades: 2,
    min_trade: 0.02,
  });
  const traded = optimisation.report.trades.trades.filter((t) => !t.cash).map((t) => t.asset).sort();
  assert.equal(traded.length, 2);
  const rebalancing = portfolio.rebalance(optimisation);
  const value = Number(rebalancing.results.value_base);
  const trades = rebalancing.results.accounts.flatMap((a) => a.trades);
  assert.deepEqual([...new Set(trades.map((t) => t.instrument))].sort(), traded);
  assert.ok(trades.every((t) => Number(t.value_base) / value > 1e-6));
  const after = rebalancing.results.weights_after;
  for (const [asset, w] of Object.entries(optimisation.results.weights)) {
    assert.ok(Math.abs((after[asset] ?? 0) - w) < 1e-9, `${asset}: ${after[asset]} for ${w}`);
  }
});

test("rebalancing to the weights held trades nothing", () => {
  const rebalancing = portfolio.rebalance(portfolio.weights());
  assert.ok(rebalancing.results.accounts.every((a) => a.trades.length === 0 && a.conversions.length === 0));
});

test("a change too small to settle is no trade", () => {
  let sign = 1;
  const nudged = {};
  for (const [key, w] of Object.entries(portfolio.weights())) {
    sign = -sign;
    nudged[key] = key.endsWith(" cash") ? w : w + sign * 1e-11;
  }
  assert.ok(portfolio.rebalance(nudged).results.accounts.every((a) => a.trades.length === 0));
  const moved = { ...portfolio.weights() };
  moved.VTI -= 1e-6;
  moved.BND += 1e-6;
  assert.ok(portfolio.rebalance(moved).results.accounts.reduce((s, a) => s + a.trades.length, 0) >= 2);
});
