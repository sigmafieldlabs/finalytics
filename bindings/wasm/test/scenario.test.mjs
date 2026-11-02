import assert from "node:assert/strict";
import { test } from "node:test";

import { engine, json, scenario } from "./setup.mjs";

const fl = await engine();
const expected = await json("tests/scenario/expected.json");

function label(key) {
  return key.holding === "instrument" ? key.id : `${key.id} cash`;
}

function results(portfolio, market) {
  const books = new fl.Books(portfolio, market);
  const dates = market.priceDates;
  const last = dates[dates.length - 1];
  const performance = books.performance(dates);
  const returns = performance.returns;
  const tail = fl.tailRisk(returns, 0.95, "cornish_fisher");
  const weights = books.weights(last);
  const held = books.holdingReturns(weights.map(([key]) => key), dates);
  const estimate = fl.covariance(held.table, { method: "ledoit_wolf" });
  const annual = { labels: estimate.matrix.labels, values: estimate.matrix.values.map((v) => v * 252) };
  const decomposition = fl.riskDecomposition(
    weights.map(([, w]) => w),
    annual,
  );
  const instruments = ["BOND", "SAP", "SPY"].map((id) => ({ holding: "instrument", id }));
  const assets = books.holdingReturns(instruments, dates);
  const sample = fl.covariance(assets.table, { method: "sample" }).matrix;
  const covariance = { labels: sample.labels, values: sample.values.map((v) => v * 252) };
  const problem = { labels: covariance.labels, covariance };
  const plan = books.rebalance(
    "main",
    [
      [instruments[2], 0.4],
      [instruments[0], 0.4],
      [instruments[1], 0.15],
    ],
    { date: last, lot_method: "highest_cost" },
  );
  return {
    valuation_total_base: books.valuation(last).total_base,
    realised_gains_base: books.realisedGains().map((g) => g.gain_base),
    time_weighted: performance.timeWeighted(),
    money_weighted: performance.moneyWeighted(),
    modified_dietz: performance.modifiedDietz(),
    returns: Array.from(returns),
    contributions: Object.fromEntries(performance.contributions().map((c) => [label(c.holding), c.contribution])),
    statistics: fl.statistics(returns),
    tail_risk: [tail.value_at_risk, tail.expected_shortfall],
    weights: Object.fromEntries(weights.map(([key, w]) => [label(key), w])),
    shrinkage: estimate.shrinkage,
    volatility: decomposition.volatility,
    risk_shares: decomposition.contributions.map((c) => c.share),
    minimum_variance: fl.solve(problem, { objective: "minimum_variance" }).weights,
    hierarchical_risk_parity: fl.solve(problem, { objective: "hierarchical_risk_parity" }).weights,
    rebalance_trades: plan.trades.map((t) => [t.instrument, t.side, t.quantity]),
    rebalance_realised_gain_base: plan.realised_gain_base,
    source_digest: books.source().digest,
  };
}

function compare(actual, wanted, path) {
  if (Array.isArray(wanted)) {
    assert.equal(actual.length, wanted.length, path);
    wanted.forEach((w, k) => compare(actual[k], w, `${path}[${k}]`));
  } else if (wanted !== null && typeof wanted === "object") {
    assert.deepEqual(Object.keys(actual).sort(), Object.keys(wanted).sort(), path);
    for (const key of Object.keys(wanted)) compare(actual[key], wanted[key], `${path}.${key}`);
  } else if (typeof wanted === "number") {
    assert.ok(Math.abs(actual - wanted) <= 1e-12 * Math.abs(wanted) || actual === wanted, `${path}: ${actual} against ${wanted}`);
  } else {
    assert.equal(actual, wanted, path);
  }
}

test("the scenario matches the Rust results", async () => {
  const { portfolio, market } = await scenario(fl);
  compare(results(portfolio, market), expected, "results");
});
