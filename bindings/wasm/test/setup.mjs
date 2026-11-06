// Loads the built package (bindings/wasm/pkg) and the shared test data.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import init, * as finalytics from "../pkg/finalytics.js";

const root = new URL("../../../", import.meta.url);

export async function engine() {
  await init({ module_or_path: await readFile(new URL("../pkg/finalytics_bg.wasm", import.meta.url)) });
  return finalytics;
}

export async function text(path) {
  return readFile(new URL(path, root), "utf8");
}

export async function json(path) {
  return JSON.parse(await text(path));
}

// Prices in CSV text, a `date` column and one column per asset, as a `PriceTable`.
export function priceTable(csv) {
  const [header, ...rows] = csv.trim().split(/\r?\n/).map((line) => line.split(","));
  return {
    dates: rows.map((row) => row[0]),
    labels: header.slice(1),
    columns: header.slice(1).map((_, j) => rows.map((row) => (row[j + 1] === "" ? null : Number(row[j + 1])))),
  };
}

export function close(actual, expected, tolerance, what) {
  const scale = Math.max(Math.abs(expected), 1);
  if (!(Math.abs(actual - expected) <= tolerance * scale)) {
    throw new Error(`${what}: ${actual} against the reference ${expected}`);
  }
}

export async function scenario(fl) {
  const loaded = fl.Portfolio.fromCsv({
    instruments: await text("tests/scenario/instruments.csv"),
    transactions: await text("tests/scenario/transactions.csv"),
    prices: await text("tests/scenario/prices.csv"),
    fx: await text("tests/scenario/fx.csv"),
    base_currency: "USD",
  });
  const market = fl.MarketData.fromCsv(await text("tests/scenario/prices.csv"), await text("tests/scenario/fx.csv"));
  assert.deepEqual(loaded.market.toDocument(), market.toDocument());
  return { portfolio: loaded.data, market };
}
