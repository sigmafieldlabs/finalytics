import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json, scenario } from "./setup.mjs";

const fl = await engine();
const reference = await json("crates/finalytics/tests/fixtures/reports.json");
const { portfolio, market } = await scenario(fl);
const books = new fl.Portfolio(portfolio, market);

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

test("table titles are Excel sheet names, and a full report gives every table", () => {
  const portfolio = fl.datasets.portfolio();
  const report = portfolio.report(fl.datasets.reportOptions());
  const tables = fl.reportTables(report);
  const parts = ["overview", "holdings", "performance", "risk", "attribution", "scenarios"];
  assert.equal(tables.length, parts.reduce((n, p) => n + fl.reportTables(report[p]).length, 0));
  const seen = new Set();
  for (const { title } of tables) {
    assert.ok(title.length <= 31 && !/[[\]:*?/\\]/.test(title), title);
    assert.ok(!title.startsWith("'") && !title.endsWith("'"), title);
    assert.ok(!seen.has(title.toLowerCase()), title);
    seen.add(title.toLowerCase());
  }
  assert.ok(tables.some((t) => t.title === "Return and risk by holding"));
});

test("a benchmark in another currency is converted, and a blend gives the attribution segments", () => {
  const files = Object.fromEntries(["transactions", "instruments", "accounts", "prices", "fx"].map((n) => [n, fl.datasets.file(`portfolio/${n}.csv`)]));
  const eur = fl.Portfolio.fromCsv({ ...files, base_currency: "EUR" });
  assert.equal(eur.overviewReport({ benchmark: "BENCH" }).provenance.benchmark, "BENCH, in USD, converted to EUR at the market data's rates");
  const report = fl.datasets.portfolio().report({ benchmark: { VTI: 0.55, BND: 0.35, GLD: 0.1 }, attribution: { scheme: "asset_class" } });
  assert.equal(report.attribution.periods.length, 36);
  const first = Object.fromEntries(report.attribution.periods[0].segments.map((s) => [s.name, s.benchmark_weight]));
  close(first.Equity, 0.55, 1e-12, "equity");
});
