// The datasets in the package: the S&P 500 files are read on demand, in Node from the package
// directory and in browsers with fetch.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

import { Assets, datasets } from "../pkg/node.js";

test("the S&P 500 prices and index load on demand", async () => {
  const prices = await datasets.sp500();
  assert.equal(prices.labels.length, 56);
  assert.equal(prices.labels.at(-1), "T-Bills");
  assert.equal(prices.dates[0], "2005-01-03");
  assert.equal(prices.dates.at(-1), "2026-10-30");
  prices.labels.length = 0;
  assert.equal((await datasets.sp500()).labels.length, 56, "each call gives its own copy");
  const index = await datasets.sp500Index();
  assert.deepEqual(index.labels, ["S&P 500"]);
  assert.equal(index.dates.length, prices.dates.length);
  const assets = new Assets(await datasets.sp500(), { risk_free_asset: "T-Bills" });
  assert.equal(assets.periodsPerYear, 252);
});

test("the companies give each stock's sector", () => {
  const companies = datasets.sp500Companies();
  assert.equal(companies.length, 55);
  assert.equal(new Set(companies.map((c) => c.sector)).size, 11);
  assert.ok(companies.every((c) => c.ticker && c.name && c.industry));
  assert.throws(() => datasets.file("sp500/prices.csv"), /on demand/);
  assert.ok(datasets.file("sp500/companies.csv").startsWith("ticker,name,sector,industry"));
});

test("browsers fetch the S&P 500 files from the package", async () => {
  globalThis.fetch = async (url) => ({ ok: true, text: () => readFile(url, "utf8") });
  const browser = await import("../pkg/index.js");
  const prices = await browser.datasets.sp500();
  assert.equal(prices.labels.length, 56);
});
