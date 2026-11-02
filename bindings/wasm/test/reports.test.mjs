import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json, scenario } from "./setup.mjs";

const fl = await engine();
const reference = await json("crates/finalytics/tests/fixtures/reports.json");
const { portfolio, market } = await scenario(fl);
const books = new fl.Books(portfolio, market);

function rectangular(tables) {
  assert.ok(tables.length > 0);
  for (const t of tables) {
    for (const row of t.rows) assert.equal(row.length, t.columns.length, t.name);
  }
}

test("every report runs and exports", () => {
  const options = { group_by: ["asset_class"], benchmark: "SPY", rolling_window: 20 };
  for (const name of ["holdingsReport", "overviewReport", "performanceReport", "riskReport"]) {
    const report = books[name](options);
    assert.deepEqual(report.provenance.source, books.source());
    rectangular(fl.reportTables(report));
  }
});

test("performance matches the reference", () => {
  const report = books.performanceReport({ benchmark: "SPY" });
  close(report.monthly.values[0][8], reference.months["2026-09"], 1e-12, "September");
  close(report.monthly.values[0][9], reference.months["2026-10"], 1e-12, "October");
  const itd = report.periods.find((p) => p.label === "ITD");
  close(itd.cumulative, reference.periods.ITD.cumulative, 1e-12, "ITD");
});

test("scenarios side by side", () => {
  const report = books.scenariosReport({
    scenarios: [
      {
        name: "Equities -20%",
        scenario: { type: "shocks", shocks: { groups: [{ scheme: "asset_class", value: "Equity", shock: -0.2 }] } },
        stress: { volatility_scale: 1.5, correlation_shift: 0.5 },
      },
      { name: "September", scenario: { type: "historical", window: { start: "2026-09-01", end: "2026-09-30" } } },
    ],
  });
  assert.deepEqual(report.changes.columns, ["Equities -20%", "September"]);
  rectangular(fl.reportTables(report));
});

test("a report without its inputs is an error", () => {
  assert.throws(() => books.attributionReport({}), (e) => e.name === "ValidationError" && e.issues[0].path === "options.attribution");
});
