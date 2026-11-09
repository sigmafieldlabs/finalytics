// Every problem with a call's inputs is reported together, each at the option given, under
// `options` for every verb: no problem hides another, and a value that cannot be read is
// reported with the problems of the others.

import assert from "node:assert/strict";
import { test } from "node:test";

import * as fl from "../pkg/node.js";

const { Assets, Universe, datasets } = fl;
const portfolio = datasets.portfolio();
const prices = datasets.assetClassPrices();
const assets = new Assets(prices, { risk_free_asset: "T-Bills" });

/** The paths of the problems `call` reports, sorted, with each path's message. */
function problems(call) {
  try {
    call();
  } catch (e) {
    assert.equal(e.name, "ValidationError", String(e));
    return Object.fromEntries(e.issues.map((i) => [i.path, i.message]));
  }
  assert.fail("the inputs were accepted");
}

function paths(call) {
  return Object.keys(problems(call)).sort();
}

const cases = [
  ["four bad options of an optimisation", () => assets.optimise({ method: "minimum_cvar", max_weight: 2, confidence: 5, asset_bounds: { Nope: { min: 0, max: 1 } }, min_effective_assets: 0 }),
    ["options.asset_bounds.Nope", "options.confidence", "options.max_weight", "options.min_effective_assets"]],
  ["a number that is not finite and an unknown asset", () => assets.optimise({ method: "minimum_variance", max_weight: NaN, asset_bounds: { Nope: { min: 0, max: 1 } } }),
    ["options.asset_bounds.Nope", "options.max_weight"]],
  ["an unknown method and a limit out of range", () => assets.optimise({ method: "minimum_varience", max_weight: 2 }),
    ["options.max_weight", "options.method"]],
  ["an unknown method does not make its parameters unused", () => assets.optimise({ method: "minimum_varience", confidence: 0.9, max_weight: 2 }),
    ["options.max_weight", "options.method"]],
  ["weights, a limit without the frontier and a rate that is not finite", () => assets.evaluate({ weights: { Nope: 0.5, Bonds: 0.6 }, max_weight: 0.4, risk_free_rate: NaN }),
    ["options.max_weight", "options.risk_free_rate", "options.weights", "options.weights.Nope"]],
  ["a backtest's strategy and settings", () => assets.backtest({ method: "minimum_variance", max_weight: 2, trading_cost_bps: -5, validation: { holdout: { start: "2099-01-01" } } }),
    ["options.max_weight", "options.trading_cost_bps", "options.validation.start"]],
  ["fixed weights with an unknown asset and limits", () => assets.backtest({ weights: { Nope: 0.6, Bonds: 0.4 }, max_weight: 0.5 }),
    ["options.max_weight", "options.weights.Nope"]],
  ["a comparison's strategies and benchmark, where they were given", () => assets.compare({ methods: ["minimum_variance", { name: "X", weights: { Nope: 1 } }], max_weight: 2, benchmark: { Nope2: 1 } }),
    ["options.benchmark.Nope2", "options.max_weight", "options.methods[1].weights.Nope"]],
  ["an unknown method among good ones", () => assets.compare({ methods: ["minimum_variance", "minimum_varience"], max_weight: 2 }),
    ["options.max_weight", "options.methods[1]"]],
  ["the frontier's limits", () => assets.frontier(5, { max_weight: 2, cash: { min: 0.5, max: 0.1 } }),
    ["options.cash", "options.max_weight"]],
  ["a rebalancing's targets and choices", () => portfolio.rebalance({ Nope: 0.5, VTI: 0.7 }, { fee_bps: -1, tax: [2, -1], accounts: ["nope"] }),
    ["options.accounts[0]", "options.fee_bps", "options.tax[0]", "options.tax[1]", "targets", "targets.Nope"]],
  ["a report's options", () => portfolio.report({ group_by: ["nope"], confidences: [2], as_of: "1999-01-01", benchmark: "NOPE", accounts: ["nope"] }),
    ["options.accounts[0]", "options.as_of", "options.benchmark", "options.confidences[0]", "options.group_by[0]"]],
  ["a plan's settings", () => assets.plan({ weights: { Nope: 1 }, inflation: 0.02, end: "2030-13", simulations: 0, chance: 2 }),
    ["options.chance", "options.end", "options.goal", "options.simulations", "options.weights.Nope"]],
  ["a screen's rules, sort, formulas and limit", () => new Universe([{ id: "A", dates: ["2024-01-02", "2024-01-03"], close: [1, 2] }]).screen({ rules: [{ measure: "nope", op: ">", value: 1 }], sort: { measure: "nope2", order: "descending" }, formulas: { f: "nope3 / 2" }, limit: -1 }),
    ["options.formulas.f", "options.limit", "options.rules[0].measure", "options.sort.measure"]],
  ["the settings of assets", () => new Assets(prices, { risk_free_asset: "Nope", periods_per_year: -1, covariance: "nope" }),
    ["options.covariance", "options.periods_per_year", "options.risk_free_asset"]],
];

for (const [name, call, expected] of cases) {
  test(`every problem together: ${name}`, () => {
    assert.deepEqual(paths(call), expected);
  });
}

test("problems are worded alike in every language", () => {
  const found = problems(() => assets.optimise({ method: "minimum_varience", max_wieght: 0.4, max_weight: NaN }));
  assert.match(found["options.method"], /^minimum_varience is not one of minimum_variance, /);
  assert.equal(found["options.max_wieght"], "is not an option; did you mean max_weight?");
  assert.equal(found["options.max_weight"], "must be a finite number, not NaN");
  const limit = problems(() => new Universe([{ id: "A", dates: ["2024-01-02", "2024-01-03"], close: [1, 2] }]).screen({ limit: -1 }));
  assert.equal(limit["options.limit"], "must be a whole number of zero or more, not -1");
  const fees = problems(() => portfolio.rebalance({ VTI: 1 }, { fees: { rate: "0.001" } }));
  assert.deepEqual(Object.keys(fees), ["options.fees"]);
});
