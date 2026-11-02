import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json } from "./setup.mjs";

const fl = await engine();
const bl = await json("crates/finalytics-opt/tests/fixtures/black_litterman.json");
const opt = await json("crates/finalytics-opt/tests/fixtures/optimisation.json");

function closeAll(actual, expected, tolerance, what) {
  assert.equal(actual.length, expected.length, `${what}: lengths differ`);
  const scale = Math.max(...expected.map(Math.abs));
  actual.forEach((a, k) => {
    assert.ok(Math.abs(a - expected[k]) <= tolerance * scale, `${what}[${k}]: ${a} against ${expected[k]}`);
  });
}

test("Black-Litterman matches PyPortfolioOpt", () => {
  for (const c of bl.cases) {
    const estimate = fl.blackLitterman({
      covariance: { labels: bl.labels, values: bl.covariance },
      prior: c.prior,
      views: c.views,
      tau: c.tau,
    });
    closeAll(estimate.posterior_returns, c.posterior_returns, 1e-13, `${c.name} returns`);
    closeAll(estimate.posterior_covariance.values, c.posterior_covariance, 1e-13, `${c.name} covariance`);
  }
  const f = bl.implied_risk_aversion;
  close(
    fl.impliedRiskAversion(f.returns, { periods_per_year: f.periods_per_year, risk_free_rate: f.risk_free_rate }),
    f.risk_aversion,
    1e-12,
    "risk aversion",
  );
});

const base = {
  labels: opt.labels,
  covariance: { labels: opt.labels, values: opt.covariance },
  expected_returns: opt.expected_returns,
};

test("robust mean-variance matches CVXPY", () => {
  for (const c of opt.robust) {
    const solution = fl.solve({ ...base, constraints: c.constraints }, c.problem);
    assert.equal(solution.status, "optimal", c.name);
    close(solution.objective_value, c.objective_value, 1e-7, `${c.name} objective`);
    solution.weights.forEach((w, k) => assert.ok(Math.abs(w - c.weights[k]) < 1e-4, `${c.name} weight ${k}`));
  }
});

test("resampled optimisation matches the reference", () => {
  const c = opt.resampled;
  const found = fl.resampled({ ...base, constraints: c.constraints }, c.problem, c.resampling);
  assert.equal(found.solved, c.resampling.samples);
  found.weights.forEach((w, k) => assert.ok(Math.abs(w - c.weights[k]) < 1e-4, `weight ${k}`));
});
