import assert from "node:assert/strict";
import { test } from "node:test";

import { engine } from "./setup.mjs";

const fl = await engine();

const prices = fl.datasets.assetClassPrices();
const assets = new fl.Assets(prices);
const labels = assets.labels;
const dates = assets.dates;
const columns = assets.returns;

test("Assets.fromReturns takes Float64Arrays as its types say", () => {
  assert.ok(columns[0] instanceof Float64Array);
  const typed = fl.Assets.fromReturns(dates, labels, columns, { risk_free_asset: "T-Bills" });
  const plain = fl.Assets.fromReturns(
    dates,
    labels,
    columns.map((c) => Array.from(c)),
    { risk_free_asset: "T-Bills" },
  );
  assert.deepEqual(typed.statistics(), plain.statistics());
  assert.deepEqual(typed.optimise({ method: "minimum_variance" }).results.weights,
    plain.optimise({ method: "minimum_variance" }).results.weights);
});

test("typed arrays are read wherever arrays of numbers are", () => {
  const table = { labels, columns };
  const plain = { labels, columns: columns.map((c) => Array.from(c)) };
  assert.deepEqual(fl.covariance(table, { type: "ledoit_wolf" }), fl.covariance(plain, { type: "ledoit_wolf" }));
  const matrix = fl.covariance(plain).matrix;
  const weights = new Float64Array(labels.length).fill(1 / labels.length);
  const typed = { labels: matrix.labels, values: Float64Array.from(matrix.values) };
  assert.deepEqual(fl.riskDecomposition(weights, typed), fl.riskDecomposition(Array.from(weights), matrix));
  // Other typed arrays too, such as integers.
  const counts = { labels: ["a", "b"], columns: [Int32Array.of(1, 0, 0, 2), Int32Array.of(0, 1, 1, 0)] };
  assert.deepEqual(fl.covariance(counts).matrix.values, fl.covariance({ labels: ["a", "b"], columns: [[1, 0, 0, 2], [0, 1, 1, 0]] }).matrix.values);
});

test("a number that is not finite in a typed array is reported where it is", () => {
  const bad = columns.map((c) => Float64Array.from(c));
  bad[1][3] = Number.NaN;
  assert.throws(
    () => fl.Assets.fromReturns(dates, labels, bad),
    (e) => e.name === "ValidationError" && e.issues.some((i) => i.path === "columns[1][3]"),
  );
});
