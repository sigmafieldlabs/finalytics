import assert from "node:assert/strict";
import { test } from "node:test";

// The package's entry for Node.js, whose results have their reports' methods.
import { Assets, datasets } from "../pkg/node.js";
import { close } from "./setup.mjs";

const prices = datasets.assetClassPrices();
const assets = new Assets(prices, { risk_free_asset: "T-Bills" });
const risky = assets.labels.filter((l) => l !== "T-Bills");

function paths(fn) {
  try {
    fn();
  } catch (error) {
    assert.equal(error.name, "ValidationError");
    return error.issues.map((i) => i.path);
  }
  assert.fail("expected a validation error");
}

function portfolioReturns(weights) {
  const columns = assets.returns;
  return assets.dates.map((_, t) =>
    assets.labels.reduce((sum, label, i) => sum + (weights[label] ?? 0) * columns[i][t], 0),
  );
}

test("the mean estimator sets the expected returns", () => {
  const shrunk = new Assets(prices, { risk_free_asset: "T-Bills", mean_returns: "james_stein" });
  const before = assets.meanReturns();
  const after = shrunk.meanReturns();
  const mean = (m) => risky.reduce((s, l) => s + m[l], 0) / risky.length;
  const grand = mean(before);
  close(mean(after), grand, 1e-12, "grand mean");
  for (const l of risky) assert.ok(Math.abs(after[l] - grand) <= Math.abs(before[l] - grand) + 1e-15, l);
  assert.equal(after["T-Bills"], before["T-Bills"]);
  const result = shrunk.optimise({ method: "maximum_sharpe" });
  assert.match(result.report.toText(), /James–Stein mean expected returns/);
  const weighted = new Assets(prices, { mean_returns: { type: "ewma", decay: 0.97 } });
  assert.equal(Object.keys(weighted.meanReturns()).length, 6);
  assert.deepEqual(paths(() => new Assets(prices, { mean_returns: { type: "ewma", decay: 2 } })), [
    "mean_returns.decay",
  ]);
});

test("the new covariance estimators are chosen by name", () => {
  for (const covariance of ["constant_correlation", "denoised", "nonlinear_shrinkage", { type: "statistical", factors: 2 }]) {
    const chosen = new Assets(prices, { covariance });
    assert.equal(chosen.optimise({ method: "minimum_variance" }).results.status, "optimal");
  }
  assert.throws(() => new Assets(prices, { covariance: "statistical" }), /number of factors/);
});

test("the downside methods minimise what they say", () => {
  const semi = assets.optimise({ method: "minimum_semivariance" });
  const returns = portfolioReturns(semi.results.weights);
  const below = returns.reduce((s, r) => s + Math.min(r, 0) ** 2, 0) / returns.length;
  close(semi.results.solution.objective_value, below, 1e-10, "semivariance");
  for (const method of ["minimum_cvar", "minimum_semivariance", "minimum_cdar"]) {
    const floored = assets.optimise({ method, target: 0.08 });
    assert.equal(floored.results.status, "optimal", method);
    assert.ok(floored.results.expected_return >= 0.08 - 1e-9, method);
  }
  assert.deepEqual(paths(() => assets.optimise({ method: "minimum_semivariance", confidence: 0.9 })), [
    "options.confidence",
  ]);
});

test("the effective number of assets is held at its floor", () => {
  for (const method of ["minimum_variance", "maximum_sharpe"]) {
    const result = assets.optimise({ method, min_effective_assets: 4 });
    const weights = Object.values(result.results.weights);
    assert.equal(result.results.status, "optimal", method);
    assert.ok(1 / weights.reduce((s, w) => s + w * w, 0) >= 4 - 1e-6, method);
  }
  assert.deepEqual(
    paths(() =>
      assets.optimise({
        method: "minimum_variance",
        min_effective_assets: 3,
        asset_bounds: { Gold: { min: 0.6, max: 1 } },
      }),
    ),
    ["options.min_effective_assets"],
  );
});

test("resampled weights are the average of the simulated optima", () => {
  const once = assets.optimise({ method: "maximum_sharpe", max_weight: 0.6 });
  const resampled = assets.optimise({ method: "maximum_sharpe", max_weight: 0.6, resample: 50 });
  const detail = resampled.results.resampled;
  assert.equal(detail.solved, 50);
  detail.labels.forEach((label, k) => assert.equal(resampled.results.weights[label], detail.weights[k]));
  assert.deepEqual(resampled.results.solution.weights, once.results.solution.weights);
  assert.deepEqual(paths(() => assets.optimise({ method: "equal_weight", resample: 50 })), ["options.resample"]);
});

test("walk-forward refits every so many periods", () => {
  const result = assets.backtest({
    method: "minimum_variance",
    validation: { walk_forward: { years: 5 } },
    rebalance: { every: { periods: 12 } },
  });
  const run = result.results.strategies[0].out_of_sample;
  const positions = run.trades.map((t) => run.dates.indexOf(t.date));
  assert.ok(positions.length > 0);
  positions.forEach((p, k) => assert.equal(p, 12 * (k + 1) - 1));
  assert.match(result.report.toText(), /rebalanced every 12 periods/);
  assert.deepEqual(
    paths(() => assets.backtest({ method: "minimum_variance", rebalance: { every: { periods: 0 } } })),
    ["rebalance.periods"],
  );
});
