import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json } from "./setup.mjs";

const fl = await engine();
const f = await json("crates/finalytics-risk/tests/fixtures/factors.json");
const optimisation = await json("crates/finalytics-opt/tests/fixtures/optimisation.json");

const returns = { labels: f.assets, columns: f.returns };

function closeAll(actual, expected, tolerance, what) {
  assert.equal(actual.length, expected.length, `${what}: lengths differ`);
  const scale = Math.max(...expected.map(Math.abs));
  actual.forEach((a, k) => {
    assert.ok(Math.abs(a - expected[k]) <= tolerance * scale, `${what}[${k}]: ${a} against ${expected[k]}`);
  });
}

function referenceModel() {
  const r = f.regression;
  return fl.factorModel({
    assets: f.assets,
    exposures: r.exposures,
    factor_covariance: { labels: f.factors, values: r.factor_covariance },
    specific_variances: r.specific_variances,
  });
}

test("regression, statistical and fundamental models match the references", () => {
  const regression = fl.regressionModel(returns, { labels: f.factors, columns: f.factor_returns });
  assert.equal(regression.method, "regression");
  closeAll(regression.model.exposures.flat(), f.regression.exposures.flat(), 1e-11, "exposures");
  closeAll(regression.model.specific_variances, f.regression.specific_variances, 1e-11, "specific variances");

  const statistical = fl.statisticalModel(returns, f.components);
  assert.deepEqual(statistical.model.factor_covariance.labels, ["pc1", "pc2", "pc3", "pc4"]);
  closeAll(statistical.explained_variance, f.statistical.explained_variance, 1e-11, "explained variance");
  closeAll(statistical.model.exposures.flat(), f.statistical.exposures.flat(), 1e-11, "loadings");

  const fundamental = fl.fundamentalModel(returns, f.factors, f.exposures);
  closeAll(fundamental.factor_returns.flat(), f.fundamental.factor_returns.flat(), 1e-12, "factor returns");
});

test("factor risk matches its definition", () => {
  const model = referenceModel();
  const risk = fl.factorRisk(Float64Array.from(f.weights), model);
  close(risk.volatility, f.risk.volatility, 1e-13, "volatility");
  closeAll(
    risk.factors.map((c) => c.component),
    f.risk.factor_components,
    1e-12,
    "factor components",
  );
  const active = fl.factorTrackingError(f.weights, f.benchmark, model);
  close(active.volatility, f.tracking_error.volatility, 1e-13, "tracking error");
  closeAll(fl.impliedCovariance(model).values, f.risk.covariance, 1e-14, "implied covariance");
});

test("values cross the boundary exactly", () => {
  // With one-hot weights each exposure is a single product with 1 plus sums of zeros, so it
  // equals the model's entry only if the number was read exactly.
  const model = referenceModel();
  for (const i of [0, 5, f.assets.length - 1]) {
    const weights = new Float64Array(f.assets.length);
    weights[i] = 1;
    const exposures = fl.factorRisk(weights, model).factors.map((c) => c.exposure);
    assert.deepEqual(exposures, model.exposures[i]);
  }
});

test("invalid models report each problem", () => {
  assert.throws(
    () =>
      fl.factorModel({
        assets: ["a", "b"],
        exposures: [[1], [0.5]],
        factor_covariance: { labels: ["market"], values: [0.04] },
        specific_variances: [0.01, -0.02],
      }),
    (e) => {
      assert.equal(e.name, "ValidationError");
      assert.deepEqual(
        e.issues.map((i) => i.path),
        ["model.specific_variances[1]"],
      );
      return true;
    },
  );
});

test("optimisation with a factor model", () => {
  const block = optimisation.factor_model;
  const model = fl.factorModel({
    assets: optimisation.labels,
    exposures: block.exposures,
    factor_covariance: { labels: ["f0", "f1"], values: block.factor_covariance },
    specific_variances: block.specific_variances,
  });
  const c = block.cases[0];
  const problem = {
    labels: optimisation.labels,
    factor_model: model,
    expected_returns: optimisation.expected_returns,
    constraints: c.constraints,
  };
  const solution = fl.solve(problem, c.problem);
  assert.equal(solution.status, "optimal");
  close(solution.objective_value, c.objective_value, 1e-7, "objective");
  solution.weights.forEach((w, k) => assert.ok(Math.abs(w - c.weights[k]) < 1e-4, `weight ${k}`));
});
