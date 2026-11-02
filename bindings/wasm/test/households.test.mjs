import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, json, text } from "./setup.mjs";

const fl = await engine();
const f = await json("crates/finalytics/tests/fixtures/households.json");
const books = fl.Portfolio.fromCsv({
  instruments: await text("tests/scenario/instruments.csv"),
  transactions: await text("tests/scenario/transactions.csv"),
  accounts: await text("tests/scenario/accounts.csv"),
  prices: await text("tests/scenario/prices.csv"),
  fx: await text("tests/scenario/fx.csv"),
  base_currency: "USD",
});
const portfolio = books.data;

function compare(found, expected) {
  assert.deepEqual(found.accounts, expected.accounts);
  for (const name of ["value_start", "value_end", "flows", "time_weighted", "contribution"]) {
    close(found[name], expected[name], 1e-12, `${found.name} ${name}`);
  }
  close(found.money_weighted, expected.money_weighted, 1e-10, `${found.name} money-weighted`);
}

test("accounts read from CSV keep their classifications", () => {
  assert.deepEqual(
    portfolio.accounts.map((a) => a.classifications),
    [
      { owner: "Alice", wrapper: "taxable" },
      { owner: "Alice", wrapper: "ISA" },
    ],
  );
});

test("a household matches the reference", () => {
  const found = books.household(f.dates, { group_by: ["owner", "wrapper"] });
  compare(found.household, f.household);
  found.accounts.forEach((a, k) => compare(a, f.accounts[k]));
  for (const scheme of found.groups) {
    scheme.groups.forEach((g, k) => compare(g, f.groups[scheme.scheme][k]));
  }
});

test("account series add up to the value series", () => {
  const series = books.accountSeries();
  assert.equal(series.lines.length, 2);
  assert.equal(series.dates.length, f.dates.length);
});
