// The contracts between features: each verb takes what the others give it, in every shape of
// data: the sample household's assets (five funds, euro cash, and dollar cash as the risk-free
// asset), the asset classes with T-bills as the risk-free asset, and the asset classes without
// them at a 2% rate.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

import * as fl from "../pkg/node.js";

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

/** Weights that hold `label` at 0.1 and spread the rest over the risky assets. */
function holding(assets, label) {
  const risky = assets.riskyAssets.filter((r) => r !== label);
  const weights = Object.fromEntries(risky.map((r) => [r, 0.9 / risky.length]));
  weights[label] = (weights[label] ?? 0) + 0.1;
  return weights;
}

/** A risky asset, the risk-free asset, cash in another currency, and Cash at the rate. */
const labelsOf = (assets) => [assets.riskyAssets[0], ...assets.cashAssets, ...(assets.riskFreeAsset ? [] : ["Cash"])];

function weightsInputs(assets, weights) {
  const plan = { inflation: 0.02, end: "2030-12", simulations: 100 };
  return {
    "optimise current_weights": () => assets.optimise({ method: "minimum_variance", current_weights: weights }),
    "optimise current_weights with cash": () =>
      assets.optimise({ method: "minimum_variance", cash: { min: 0, max: 0.3 }, current_weights: weights }),
    "optimise benchmark_weights": () =>
      assets.optimise({ method: "minimum_variance", benchmark_weights: weights, max_tracking_error: 0.5 }),
    "frontier current_weights": () => assets.frontier(5, { current_weights: weights, max_turnover: 0.5 }),
    "evaluate weights": () => assets.evaluate({ weights }),
    "backtest weights": () => assets.backtest({ weights }),
    "backtest benchmark": () => assets.backtest({ method: "minimum_variance", benchmark: weights }),
    "compare methods weights": () => assets.compare({ methods: ["minimum_variance", { name: "Mix", weights }] }),
    "compare weights": () => assets.compare({ methods: ["minimum_variance"], weights: { Mix: weights } }),
    "compare benchmark": () => assets.compare({ methods: ["minimum_variance"], benchmark: weights }),
    "plan weights": () => assets.plan({ weights, ...plan }),
    "plan alternatives": () => assets.plan({ weights: { [assets.riskyAssets[0]]: 1 }, alternatives: { Alt: weights }, ...plan }),
  };
}

test("every input of weights takes every label of the assets", () => {
  for (const [shape, assets] of Object.entries(shapes)) {
    for (const label of labelsOf(assets)) {
      const weights = holding(assets, label);
      for (const [name, run] of Object.entries(weightsInputs(assets, weights))) {
        assert.doesNotThrow(run, `${shape} ${name} ${label}`);
      }
      if (shape === "household") assert.doesNotThrow(() => portfolio.rebalance(weights), label);
    }
  }
});

test("every input of weights refuses an unknown label in one message", () => {
  const unknown = (e) => e.issues.some((i) => i.path.endsWith(".Nope") && i.message === "unknown asset Nope");
  for (const [shape, assets] of Object.entries(shapes)) {
    for (const [name, run] of Object.entries(weightsInputs(assets, holding(assets, "Nope")))) {
      assert.throws(run, unknown, `${shape} ${name}`);
    }
  }
  assert.throws(() => portfolio.rebalance(holding(household, "Nope")), (e) =>
    e.issues.some((i) => i.path === "targets.Nope" && i.message === "unknown asset Nope"));
});

test("bounds and groups refuse the cash in favour of cash limits", () => {
  for (const [label, says] of [["USD cash", "limit it with cash"], ["EUR cash", "cash in another currency"]]) {
    assert.throws(
      () => household.optimise({
        method: "minimum_variance",
        asset_bounds: { [label]: { min: 0, max: 0.1 } },
        groups: { G: { assets: [label, "VTI"], max: 0.9 } },
      }),
      (e) => {
        const paths = e.issues.map((i) => i.path);
        return paths.includes(`options.asset_bounds.${label}`) && paths.includes("options.groups.G") &&
          e.issues.every((i) => i.message.includes(says));
      },
    );
  }
});

test("the cash a mix holds in another currency is measured in every verb", () => {
  const mix = portfolio.weights();
  assert.ok("EUR cash" in mix);
  const valued = household.evaluate({ weights: mix });
  const backtest = household.backtest({ weights: mix });
  assert.ok(Math.abs(valued.report.history.weights.statistics.volatility - backtest.report.strategy.tested.statistics.volatility) < 1e-12);
  assert.ok(backtest.results.assets.includes("EUR cash"));
  const traded = household.optimise({ method: "minimum_variance", current_weights: mix, max_turnover: 0.3 });
  const eur = traded.report.trades.trades.find((t) => t.asset === "EUR cash");
  assert.ok(eur.cash && eur.proposed === 0 && Math.abs(eur.current - mix["EUR cash"]) < 1e-15);
  assert.ok(traded.report.trades.turnover <= 0.3 + 1e-9);
});

/** Fixed weights to echo: two risky assets with the cash, and a portfolio's own weights. */
function mixes(assets, withPortfolio) {
  const [a, b] = assets.riskyAssets;
  const cash = assets.riskFreeAsset ?? "Cash";
  const out = [{ [a]: 0.6, [b]: 0.3, [cash]: 0.1 }, { [a]: 0.5, [b]: 0.5 }];
  if (withPortfolio) out.push(portfolio.weights());
  return out;
}

test("fixed weights come back as given", () => {
  for (const [shape, assets] of Object.entries(shapes)) {
    for (const mix of mixes(assets, shape === "household")) {
      assert.deepEqual(assets.backtest({ weights: mix }).report.strategy.weights[0].weights, mix, shape);
      const compared = assets.compare({ methods: ["minimum_variance", { name: "Mix", weights: mix }] });
      assert.deepEqual(compared.report.strategies[1].weights[0].weights, mix, shape);
      const against = assets.backtest({ method: "minimum_variance", benchmark: mix });
      assert.deepEqual(against.report.benchmark.weights[0].weights, mix, shape);
      const benchmarked = assets.compare({ methods: ["minimum_variance"], benchmark: mix });
      assert.deepEqual(benchmarked.report.benchmark.weights[0].weights, mix, shape);
    }
  }
});

test("a backtest is titled with the name given", () => {
  const method = bills.backtest({ method: "minimum_variance", name: "Mine" });
  assert.equal(method.report.name, "Mine");
  assert.equal(method.report.strategy.name, "Mine");
  const fixed = bills.backtest({ weights: { "US Equity": 0.5, Bonds: 0.5 }, name: "Mine" });
  assert.equal(fixed.report.name, "Mine");
  assert.ok(`${fixed}`.includes("Mine"));
  assert.equal(bills.backtest({ method: "minimum_variance" }).report.name ?? null, null);
});

test("a portfolio's assets are documented with their cash", async () => {
  const types = await readFile(new URL("../pkg/finalytics.d.ts", import.meta.url), "utf8");
  const at = types.indexOf("  assets(): Assets;");
  assert.ok(at > 0);
  const doc = types.slice(types.lastIndexOf("/**", at), at);
  assert.ok(doc.includes("cashAssets") && doc.includes("EUR cash") && doc.includes("risk-free asset"), doc);
  assert.deepEqual(household.cashAssets, ["USD cash", "EUR cash"]);
});
