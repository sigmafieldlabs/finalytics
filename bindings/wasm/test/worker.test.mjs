import assert from "node:assert/strict";
import { test } from "node:test";
import { Worker } from "node:worker_threads";

import { createEngine } from "../pkg/client.js";
import { engine, scenario } from "./setup.mjs";

const fl = await engine();
const { portfolio, market } = await scenario(fl);

test("the worker gives the same results as the engine in the page", async () => {
  const remote = createEngine(new Worker(new URL("../pkg/worker.js", import.meta.url)));
  try {
    const returns = [0.01, -0.004, 0.007, -0.012, 0.003];
    assert.deepEqual(await remote.call("statistics", returns), fl.statistics(returns));

    const books = await remote.portfolio(portfolio, market.toDocument());
    const perf = await books.call("performance");
    const local = new fl.Portfolio(portfolio, market).performance();
    assert.deepEqual(await perf.get("returns"), local.returns);
    assert.equal(await perf.call("timeWeighted"), local.timeWeighted());
    await perf.free();
    await books.free();

    await assert.rejects(remote.call("statistics", [0.01]), { name: "ValidationError" });
  } finally {
    await remote.terminate();
  }
});

test("the worker calls static methods and keeps the objects they return", async () => {
  const remote = createEngine(new Worker(new URL("../pkg/worker.js", import.meta.url)));
  try {
    assert.deepEqual(await remote.static("datasets", "files"), fl.datasets.files());
    const csv = await remote.static("datasets", "file", "asset-classes.csv");
    assert.equal(csv, fl.datasets.file("asset-classes.csv"));

    const assets = await remote.static("Assets", "fromCsv", csv);
    assert.equal(typeof assets.handle, "number");
    assert.deepEqual(await assets.get("labels"), fl.Assets.fromCsv(csv).labels);
    await assets.free();

    const market = await remote.static("MarketData", "fromCsv", "date,X\n2026-01-02,10\n");
    assert.equal(typeof market.handle, "number");
    await market.free();

    const universe = await remote.static("datasets", "universe");
    assert.equal(await universe.get("length"), 201);
    const screen = await universe.call("screen", { limit: 5 });
    assert.equal(screen.results.matches.length, 5);
    await universe.free();
    await assert.rejects(remote.static("datasets", "nothing"), /no static method datasets\.nothing/);
    await assert.rejects(remote.static("Window", "open"), /no static method Window\.open/);
  } finally {
    await remote.terminate();
  }
});

test("the worker holds universes and screens them as the page does", async () => {
  const remote = createEngine(new Worker(new URL("../pkg/worker.js", import.meta.url)));
  try {
    const days = Int32Array.from({ length: 300 }, (_, k) => 20_000 + k);
    const close = (drift) => Float64Array.from({ length: 300 }, (_, k) => 100 * (1 + drift) ** k * (1 + 0.01 * Math.sin(k)));
    const series = [
      { id: "UP", dates: days, close: close(0.001), fields: { sector: "Energy" } },
      { id: "FLAT", dates: days, close: close(0), fields: { sector: "Utilities" } },
    ];
    const options = { rules: [{ field: "sector", op: "in", values: ["Energy"] }] };
    const universe = await remote.universe(series, { risk_free_rate: 0.02 });
    assert.deepEqual(await universe.get("ids"), ["UP", "FLAT"]);
    const screen = await universe.call("screen", options);
    const local = new fl.Universe(series, { risk_free_rate: 0.02 }).screen(options);
    assert.ok(screen.results.measures.return_1w instanceof Float64Array);
    assert.deepEqual(screen.results.matches, ["UP"]);
    assert.deepEqual(screen.results.measures, local.results.measures);
    assert.deepEqual(screen.report, local.report);
    await universe.free();

    const columns = { ids: ["UP"], starts: [0], dates: days, close: close(0.001) };
    const fromColumns = await remote.static("Universe", "fromColumns", columns);
    assert.equal(typeof fromColumns.handle, "number");
    assert.equal(await fromColumns.get("length"), 1);
    await fromColumns.free();
  } finally {
    await remote.terminate();
  }
});
