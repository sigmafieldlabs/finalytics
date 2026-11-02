import assert from "node:assert/strict";
import { test } from "node:test";

import { engine, json, scenario } from "./setup.mjs";

const fl = await engine();
const expected = await json("tests/scenario/expected.json");

function results(portfolio, market) {
  const books = new fl.Portfolio(portfolio, market);
  const dates = market.priceDates;
  const last = dates[dates.length - 1];
  const performance = books.performance(dates);
  const returns = performance.returns;
  const tail = fl.tailRisk(returns, 0.95, "cornish_fisher");
  const weights = books.weights(last);
  const held = books.holdingReturns(Object.keys(weights), dates);
  const estimate = fl.covariance(held.table, { type: "ledoit_wolf" });
  const annual = { labels: estimate.matrix.labels, values: estimate.matrix.values.map((v) => v * 252) };
  const decomposition = fl.riskDecomposition(
    Object.values(weights),
    annual,
  );
  const instruments = ["BOND", "SAP", "SPY"];
  const assets = books.holdingReturns(instruments, dates);
  const sample = fl.covariance(assets.table, { type: "sample" }).matrix;
  const covariance = { labels: sample.labels, values: sample.values.map((v) => v * 252) };
  const problem = { labels: covariance.labels, covariance };
  const [plan] = books.rebalance(
    { SPY: 0.4, BOND: 0.4, SAP: 0.15 },
    { accounts: ["main"], date: last, lot_method: "highest_cost" },
  ).accounts;
  return {
    valuation_total_base: books.valuation(last).total_base,
    realised_gains_base: books.realisedGains().map((g) => g.gain_base),
    time_weighted: performance.timeWeighted(),
    money_weighted: performance.moneyWeighted(),
    modified_dietz: performance.modifiedDietz(),
    returns: Array.from(returns),
    contributions: Object.fromEntries(performance.contributions().map((c) => [c.holding, c.contribution])),
    statistics: fl.statistics(returns),
    tail_risk: [tail.value_at_risk, tail.expected_shortfall],
    weights,
    shrinkage: estimate.shrinkage,
    volatility: decomposition.volatility,
    risk_shares: decomposition.contributions.map((c) => c.share),
    minimum_variance: fl.solve(problem, { type: "minimum_variance" }).weights,
    hierarchical_risk_parity: fl.solve(problem, { type: "hierarchical_risk_parity" }).weights,
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
