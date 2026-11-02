import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json } from "./setup.mjs";

const fl = await engine();
const f = await json("crates/finalytics-perf/tests/fixtures/attribution.json");

test("Brinson-Fachler matches the reference with both linkings", () => {
  const c = f.brinson_fachler;
  for (const linking of ["carino", "menchero"]) {
    const found = fl.brinsonFachler(c.periods_input, linking);
    close(found.portfolio_return, c.portfolio_return, 1e-14, "portfolio return");
    found.periods.forEach((p, k) => {
      close(p.linking_coefficient, c[linking].coefficients[k], 1e-13, `${linking} coefficient ${k}`);
    });
    found.segments.forEach((s, k) => {
      assert.ok(Math.abs(s.allocation - c[linking].allocation[k]) < 1e-15, `${linking} allocation ${k}`);
      assert.ok(Math.abs(s.selection - c[linking].selection[k]) < 1e-15, `${linking} selection ${k}`);
    });
  }
  assert.equal(fl.brinsonFachler(c.periods_input).linking, "carino");
});

test("Karnosky-Singer matches the reference", () => {
  const c = f.karnosky_singer;
  const found = fl.karnoskySinger(c.periods_input);
  found.currencies.forEach((x, k) => {
    assert.ok(Math.abs(x.allocation - c.carino.currency[k]) < 1e-15, `currency ${k}`);
  });
  const total = found.allocation + found.selection + found.interaction + found.currency + found.residual;
  assert.ok(Math.abs(total - found.excess_return) < 1e-15);
});
