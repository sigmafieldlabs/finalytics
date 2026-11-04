import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json } from "./setup.mjs";

const fl = await engine();
const statisticsFixture = await json("crates/finalytics-risk/tests/fixtures/statistics.json");
const covarianceFixture = await json("crates/finalytics-risk/tests/fixtures/covariance.json");
const downsideFixture = await json("crates/finalytics-risk/tests/fixtures/downside.json");
const optimisationFixture = await json("crates/finalytics-opt/tests/fixtures/optimisation.json");

test("statistics match the reference", () => {
  for (const c of statisticsFixture.cases) {
    const returns = Float64Array.from(c.returns);
    const stats = fl.statistics(returns, c.options);
    for (const [field, expected] of Object.entries(c.statistics)) {
      close(stats[field], expected, 1e-10, `${c.name} ${field}`);
    }
    const relative = fl.relativeStatistics(c.returns, c.benchmark, c.options);
    for (const [field, expected] of Object.entries(c.relative)) {
      close(relative[field], expected, 1e-10, `${c.name} ${field}`);
    }
    for (const tail of c.tail) {
      for (const method of ["historical", "parametric", "cornish_fisher"]) {
        const found = fl.tailRisk(c.returns, tail.confidence, method);
        close(found.value_at_risk, tail[method][0], 1e-10, `${c.name} ${method} VaR`);
        close(found.expected_shortfall, tail[method][1], 1e-9, `${c.name} ${method} ES`);
      }
    }
  }
});

test("covariance estimators match the reference", () => {
  for (const c of covarianceFixture.cases) {
    const labels = c.columns.map((_, i) => `asset ${i}`);
    const returns = { labels, columns: c.columns };
    const methods = {
      sample: { type: "sample" },
      ewma: { type: "ewma", decay: c.ewma.decay },
      ledoit_wolf: { type: "ledoit_wolf" },
      oas: { type: "oas" },
    };
    for (const [key, method] of Object.entries(methods)) {
      const estimate = fl.covariance(returns, method);
      const expected = key === "sample" ? c.sample : c[key].matrix;
      const scale = Math.max(...expected.map(Math.abs));
      estimate.matrix.values.forEach((v, k) => {
        assert.ok(Math.abs(v - expected[k]) <= 1e-12 * scale, `${c.name} ${key}[${k}]`);
      });
    }
  }
});

test("optimisation matches the reference", () => {
  const f = optimisationFixture;
  const base = {
    labels: f.labels,
    covariance: { labels: f.labels, values: f.covariance },
    expected_returns: f.expected_returns,
    scenarios: { labels: f.labels, columns: f.scenarios },
  };
  for (const c of f.cases) {
    const solution = fl.solve({ ...base, constraints: c.constraints }, c.problem);
    assert.equal(solution.status, "optimal", c.name);
    close(solution.objective_value, c.objective_value, 1e-7, c.name);
    if (c.problem.objective !== "minimum_cvar") {
      solution.weights.forEach((w, k) => assert.ok(Math.abs(w - c.weights[k]) < 2e-5, `${c.name} weight ${k}`));
    }
  }
  const frontier = fl.frontier(base, 5);
  assert.equal(frontier.length, 5);
});

test("validation errors list every issue", () => {
  assert.throws(
    () => fl.statistics([0.01, Number.NaN, -2]),
    (error) => {
      assert.equal(error.name, "ValidationError");
      assert.deepEqual(
        error.issues.map((i) => i.path),
        ["returns[1]", "returns[2]"],
      );
      return true;
    },
  );
  assert.throws(() => fl.solve({ labels: ["a"] }, { type: "mean_variance", risk_aversion: -1 }), {
    name: "ValidationError",
  });
});

test("normal distribution and drawdowns", () => {
  close(fl.normalQuantile(0.975), 1.959963984540054, 1e-15, "quantile");
  const found = fl.drawdowns(["2026-01-02", "2026-01-05", "2026-01-06"], [100, 90, 95]);
  assert.equal(found.length, 1);
  assert.equal(found[0].recovery, null);
  close(found[0].depth, -0.1, 1e-15, "depth");
});

test("downside measures match the reference", () => {
  statisticsFixture.cases.forEach((c, k) => {
    const expected = downsideFixture.cases[k];
    const stats = fl.statistics(c.returns, c.options);
    close(stats.conditional_drawdown_at_risk, expected.conditional_drawdown_at_risk, 1e-10, `${c.name} CDaR`);
    close(stats.ulcer_index, expected.ulcer_index, 1e-12, `${c.name} Ulcer index`);
    const relative = fl.relativeStatistics(c.returns, c.benchmark, c.options);
    for (const [field, value] of Object.entries(expected.relative)) {
      close(relative[field], value, 1e-10, `${c.name} ${field}`);
    }
  });
  const s = downsideFixture.shortfall;
  const found = fl.shortfallDecomposition(s.weights, { labels: s.labels, columns: s.scenarios });
  close(found.expected_shortfall, s.expected_shortfall, 1e-12, "expected shortfall");
  found.contributions.forEach((c, i) => close(c.component, s.contributions[i], 1e-8, c.label));
});
