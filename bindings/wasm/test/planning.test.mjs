// Plans, as the package's entry for Node.js gives them: a goal, the money paid in and out, the mix
// and the inflation, from plain objects; the result has a report and the raw results.
import assert from "node:assert/strict";
import { test } from "node:test";

import { Assets, Portfolio, datasets } from "../pkg/node.js";

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

// A price index rising 2.5% a year, on the 28th of each month from January 1990 to September 2026,
// without a level for October 2025.
function steadyIndex() {
  const months = [...Array(441).keys()].filter((k) => !(1990 + Math.floor(k / 12) === 2025 && k % 12 === 9));
  const date = (k) => `${1990 + Math.floor(k / 12)}-${String((k % 12) + 1).padStart(2, "0")}-28`;
  return { dates: months.map(date), labels: ["Prices"], columns: [months.map((k) => 100 * 1.025 ** (k / 12))] };
}

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
    inflation: steadyIndex(),
    model: "normal",
  });
  assert.equal(plan.results.chance_wanted, 0.75);
  assert.equal(plan.results.spare_from, 0.9);
  assert.deepEqual(plan.results.alternatives.map((a) => a.name), ["Cautious"]);
  assert.equal(plan.report.context.mix_name, "Balanced");
  assert.equal(plan.report.context.inflation.type, "index");
  assert.equal(plan.report.context.inflation.name, "Prices");
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

test("goals kept with a portfolio plan as they are", () => {
  const goals = [
    "goal,type,amount,by,every,name,weights,chance,start_value,inflation,simulations",
    "House,target,150000,2030-06,,,,0.8,60000,0.025,1000",
    "House,saving,2500,,month,,,,,,",
    "House,mix,,,,Balanced,US Equity=0.6;Bonds=0.4,,,,",
  ].join("\n");
  const file = (name) => datasets.file(`portfolio/${name}.csv`);
  const portfolio = Portfolio.fromCsv({
    transactions: file("transactions"),
    instruments: file("instruments"),
    accounts: file("accounts"),
    prices: file("prices"),
    fx: file("fx"),
    goals,
  });
  const [goal] = portfolio.goals;
  assert.equal(goal.name, "House");
  const stored = assets.plan(goal);
  const read = Portfolio.fromJson(portfolio.toJson());
  assert.deepEqual(read.goals, portfolio.goals);
  assert.deepEqual(assets.plan(read.goals[0]).results, stored.results);
  assert.deepEqual(datasets.portfolio().withGoals(read.goals).goals, read.goals);
});
