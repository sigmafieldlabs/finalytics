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
