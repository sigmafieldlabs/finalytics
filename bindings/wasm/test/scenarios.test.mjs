import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json, scenario } from "./setup.mjs";

const fl = await engine();
const f = await json("crates/finalytics/tests/fixtures/scenarios.json");
const { portfolio, market } = await scenario(fl);
const books = new fl.Books(portfolio, market);

function compare(result, expected, what) {
  close(result.value_before, expected.value_before, 1e-12, `${what} value before`);
  close(result.value_after, expected.value_after, 1e-12, `${what} value after`);
  assert.deepEqual(
    result.holdings.map((h) => h.holding),
    expected.holdings.map((h) => h.label),
  );
  result.holdings.forEach((h, k) => {
    for (const name of ["return_local", "currency_return", "return_base", "value_after"]) {
      close(h[name], expected.holdings[k][name], 1e-12, `${what} ${h.holding} ${name}`);
    }
  });
}

test("shocks match the reference", () => {
  const c = f.shocks;
  const result = books.shockScenario(c.shocks, { date: c.as_of, ...c.options });
  compare(result, c.expected, "shocks");
  for (const key of ["risk_before", "risk_after"]) {
    close(result[key].volatility, c.expected[key].volatility, 1e-12, `${key} volatility`);
    close(result[key].value_at_risk_base, c.expected[key].value_at_risk, 1e-12, `${key} value at risk`);
  }
  assert.deepEqual(result.holdings.find((h) => h.holding === "SPY").source, {
    type: "group",
    scheme: "asset_class",
    value: "Equity",
  });
});

test("factor shocks and historical replays match the reference", () => {
  for (const key of ["factors_one", "factors_two", "factors_alone"]) {
    const c = f[key];
    const result = books.shockScenario(c.shocks, { date: c.as_of });
    compare(result, c.expected, key);
    Object.values(result.factor_returns).forEach((value, k) => close(value, c.expected.factor_returns[k], 1e-12, key));
  }
  for (const key of ["historical", "historical_proxy"]) {
    const c = f[key];
    const result = books.historicalScenario(c.window, { date: c.as_of });
    compare(result, c.expected, key);
    assert.deepEqual(result.path.dates, c.expected.path.dates);
    assert.equal(result.path.worst_drawdown.trough, c.expected.path.trough);
  }
});

test("overlapping group shocks are reported", () => {
  const groups = [
    { scheme: "asset_class", value: "Equity", shock: -0.2 },
    { scheme: "region", value: "Europe", shock: -0.3 },
  ];
  assert.throws(
    () => books.shockScenario({ groups }),
    (e) => e.name === "ValidationError" && e.issues[0].path === "shocks.groups",
  );
});

test("a stressed covariance", () => {
  const stressed = fl.stressedCovariance(
    { labels: ["a", "b"], values: [0.04, 0, 0, 0.09] },
    { volatility_scale: 2, correlation_shift: 1 },
  );
  stressed.values.forEach((v, k) => close(v, 4 * [0.04, 0.06, 0.06, 0.09][k], 1e-15, `value ${k}`));
});
