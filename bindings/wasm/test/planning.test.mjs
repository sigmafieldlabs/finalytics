// Plans, as the package's entry for Node.js gives them: a goal, the money paid in and out, the mix
// and the inflation, from plain objects; the result has a report and the raw results.
import assert from "node:assert/strict";
import { test } from "node:test";

import { Assets, datasets } from "../pkg/node.js";

const assets = new Assets(datasets.assetClassPrices());

const house = {
  goal: { type: "target", amount: 150000, by: "2030-06" },
  name: "House deposit",
  start_value: 60000,
  flows: [
    { type: "saving", amount: 2500, every: "month" },
    { type: "spending", name: "Car", amount: 25000, on: "2028-03" },
  ],
  weights: { "US Equity": 0.6, Bonds: 0.4 },
  inflation: 0.025,
  fee: 0.0025,
  simulations: 500,
  currency: "USD",
};

test("a plan has a report and the raw results", () => {
  const plan = assets.plan(house);
  assert.deepEqual(Object.keys(plan), ["report", "results"]);
  const r = plan.results;
  assert.ok(r.outcome.chance > 0 && r.outcome.chance < 1);
  assert.equal(r.start, "2026-11");
  assert.equal(plan.report.context.goal.name, "House deposit");
  const text = `${plan}`;
  assert.ok(text.includes("FINALYTICS  ·  PLANNING"));
  assert.ok(text.split("\n").every((line) => [...line].length <= 80));
  assert.ok(plan.toHtml().startsWith("<!doctype html>"));
  assert.ok(`${plan.results}`.startsWith("PlanResults"));
  assert.deepEqual(assets.plan(house).results, r);
});

test("plain choices: a chance and its room, mixes by name, a price index and a model", () => {
  const plan = assets.plan({
    ...house,
    chance: [0.75, 0.9],
    weights: { type: "fixed", name: "Balanced", weights: { "US Equity": 0.6, Bonds: 0.4 } },
    alternatives: { Cautious: { "US Equity": 0.3, Bonds: 0.7 } },
    inflation: datasets.usCpi(),
    model: "normal",
  });
  assert.equal(plan.results.chance_wanted, 0.75);
  assert.equal(plan.results.spare_from, 0.9);
  assert.deepEqual(plan.results.alternatives.map((a) => a.name), ["Cautious"]);
  assert.equal(plan.report.context.mix_name, "Balanced");
  assert.equal(plan.report.context.inflation.type, "index");
  assert.equal(plan.report.context.model.type, "normal");
});

test("problems are reported with where they are", () => {
  assert.throws(
    () => assets.plan({ ...house, fee: 2, weights: { Nope: 1 } }),
    (e) => e.name === "ValidationError" && e.issues.some((i) => i.path === "fee"),
  );
  assert.throws(
    () => assets.plan({ ...house, colour: "blue" }),
    (e) => e.name === "ValidationError" && e.issues.some((i) => i.path === "options.colour"),
  );
});
