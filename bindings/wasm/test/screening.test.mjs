// Screens, as the package's entry for Node.js gives them: a universe from flat typed arrays,
// from series or from CSV, with fields; the result has a report and the raw results as columns,
// one Float64Array per measure.
import assert from "node:assert/strict";
import { test } from "node:test";

import { Universe, reportTables } from "../pkg/node.js";
import { close, json, priceTable, text } from "./setup.mjs";

const fixture = await json("crates/finalytics/tests/fixtures/screening.json");
const prices = priceTable(await text("tests/sp500/prices.csv"));
const index = priceTable(await text("tests/sp500/index.csv"));
const sectors = Object.fromEntries(
  (await text("tests/sp500/companies.csv"))
    .trim()
    .split(/\r?\n/)
    .slice(1)
    .map((line) => {
      const [ticker, , sector] = line.match(/("[^"]*"|[^,]+)/g);
      return [ticker, sector];
    }),
);

/** Days since 1970-01-01 of an ISO date. */
const day = (iso) => Date.UTC(+iso.slice(0, 4), +iso.slice(5, 7) - 1, +iso.slice(8, 10)) / 86_400_000;

/** The fixture's universe, as flat columns: the S&P 500 companies of the recipe on their own dates. */
function columns() {
  const ids = [];
  const starts = [];
  const days = [];
  const closes = [];
  const volume = [];
  const dividends = [];
  const sector = [];
  const pe = [];
  const add = (id, dates, values, recipe = {}, s = 0) => {
    ids.push(id);
    starts.push(days.length);
    let k = 0;
    for (let j = 0; j < dates.length; j++) {
      const d = dates[j];
      if (values[j] === null) continue;
      if (recipe.start && d < recipe.start) continue;
      if (recipe.end && d > recipe.end) continue;
      if ((recipe.gaps ?? []).some(([a, b]) => d >= a && d <= b)) continue;
      days.push(day(d));
      closes.push((recipe.missing ?? []).includes(d) ? NaN : values[j]);
      const unknownVolume = !recipe.volume || (recipe.volume_missing ?? []).includes(d);
      volume.push(unknownVolume ? NaN : 1_000_000 + ((k * 7919 + s * 104729) % 1000) * 1000);
      dividends.push(!recipe.dividends ? NaN : (k + s) % 63 === 0 ? 0.25 * (1 + (s % 3)) : 0);
      k++;
    }
  };
  fixture.recipe.forEach((recipe, s) => {
    add(recipe.id, prices.dates, prices.columns[prices.labels.indexOf(recipe.id)], recipe, s);
    sector.push(fixture.no_sector.includes(recipe.id) ? null : sectors[recipe.id]);
    pe.push(fixture.no_pe.includes(recipe.id) ? NaN : 8 + ((s * 37) % 29));
  });
  add(fixture.benchmark, index.dates, index.columns[0]);
  sector.push(null);
  pe.push(NaN);
  return {
    ids,
    starts: Int32Array.from(starts),
    dates: Int32Array.from(days),
    close: Float64Array.from(closes),
    volume: Float64Array.from(volume),
    dividends: Float64Array.from(dividends),
    fields: { sector, pe_ratio: Float64Array.from(pe) },
  };
}

const universe = Universe.fromColumns(columns(), {
  benchmark: fixture.benchmark,
  risk_free_rate: fixture.risk_free_rate,
});

test("a screen from typed arrays matches the reference", () => {
  assert.equal(universe.length, 21);
  assert.deepEqual(universe.fields, ["sector", "pe_ratio"]);
  for (const c of fixture.cases) {
    const screen = universe.screen(c.options);
    assert.deepEqual(Object.keys(screen), ["report", "results"]);
    const r = screen.results;
    assert.deepEqual(r.matches, c.matches, c.name);
    assert.deepEqual(Object.keys(r.measures), c.measures);
    assert.deepEqual(
      r.rules.map((x) => [x.passed, x.lacking, x.remaining]),
      c.rules.map((x) => [x.passed, x.lacking, x.remaining]),
    );
    c.securities.forEach((e, i) => {
      assert.equal(r.ids[i], e.id);
      assert.equal(r.status[i], e.status, `${c.name}: ${e.id}`);
      assert.equal(Number.isNaN(r.rank[i]) ? null : r.rank[i], e.rank);
      c.measures.forEach((m, k) => {
        const actual = r.measures[m][i];
        if (e.values[k] === null) {
          assert.ok(Number.isNaN(actual), `${c.name}: ${e.id} ${m} is ${actual}`);
        } else {
          close(actual, e.values[k], 1e-9, `${c.name}: ${e.id} ${m}`);
        }
      });
    });
  }
});

test("a screen prints, lays out as a page and turns into tables", () => {
  const screen = universe.screen({
    rules: [
      { measure: "return_1y", op: ">", value: 0 },
      { field: "sector", op: "not_in", values: ["Utilities"] },
    ],
    sort: { measure: "sharpe_ratio", order: "descending" },
    limit: 5,
    name: "Momentum",
  });
  const r = screen.results;
  assert.ok(r.measures.return_1y instanceof Float64Array);
  assert.ok(r.rank instanceof Float64Array);
  assert.ok(r.fields.pe_ratio instanceof Float64Array);
  assert.equal(r.fields.sector[r.ids.indexOf("AAPL")], "Information Technology");
  assert.equal(r.matches.length, 5);
  const page = `${screen}`;
  assert.ok(page.includes("FINALYTICS  ·  SCREEN") && page.includes("Momentum"));
  assert.ok(page.split("\n").every((line) => line.length <= 80));
  assert.ok(r.toText().startsWith("ScreenResults · as of 2026-10-30 · 5 of 21 securities matched"));
  assert.ok(screen.toHtml().startsWith("<!doctype html>"));
  assert.deepEqual(
    reportTables(screen.report).map((t) => t.name),
    ["coverage", "rules", "matches", "spread"],
  );
  // The results are data: they pass to JSON, with NaN as null.
  assert.ok(JSON.stringify(screen.report).length > 0);
});

test("a universe from series and from CSV, with fields", () => {
  const dates = prices.dates.slice(-300);
  const series = ["AAPL", "MSFT"].map((id) => ({
    id,
    dates: id === "AAPL" ? dates : Int32Array.from(dates.map(day)),
    close: Float64Array.from(prices.columns[prices.labels.indexOf(id)].slice(-300)),
    fields: { sector: sectors[id] },
  }));
  const u = new Universe(series, { risk_free_rate: 0.02 });
  assert.deepEqual(u.ids, ["AAPL", "MSFT"]);
  assert.equal(u.riskFreeRate, 0.02);
  assert.equal(u.latestDate, "2026-10-30");
  const withFields = u.withFields({ AAPL: { pe_ratio: 31.2 } });
  assert.deepEqual(withFields.fields, ["sector", "pe_ratio"]);
  const csv = "id,date,close\nA,2026-10-28,10\nA,2026-10-29,10.5\nB,2026-10-28,19\nB,2026-10-29,20\nA,2026-10-30,11\n";
  const fromCsv = Universe.fromCsv(csv, { benchmark: "A" }).withFieldsCsv("id,sector\nA,Energy\nB,Utilities\n");
  assert.equal(fromCsv.benchmark, "A");
  const screen = fromCsv.screen({ rules: [{ field: "sector", op: "in", values: ["Energy"] }] });
  assert.deepEqual(screen.results.matches, ["A"]);
  assert.deepEqual(screen.results.status, ["measured", "measured"]);
});

test("every problem is reported together", () => {
  assert.throws(
    () =>
      universe.screen({
        measures: ["return_2y"],
        rules: [
          { measure: "returns_1y", op: ">", value: 0 },
          { field: "sector", op: ">", value: 1 },
          { measure: "rsi_14", op: "between", value: [70, 30] },
        ],
        sort: { field: "sector", order: "descending" },
        limit: 0,
      }),
    (error) => {
      assert.equal(error.name, "ValidationError");
      assert.deepEqual(
        error.issues.map((i) => i.path),
        ["measures[0]", "rules[0].measure", "rules[1].op", "rules[2].value", "sort.field", "limit"],
      );
      return true;
    },
  );
  assert.throws(() => universe.screen({ colour: "red" }), /options.colour/);
  assert.throws(
    () => Universe.fromColumns({ ids: ["A"], starts: [0, 1], dates: ["2026-10-30"], close: new Float64Array([1]) }),
    { name: "ValidationError" },
  );
});
