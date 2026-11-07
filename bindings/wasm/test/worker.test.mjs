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

    await assert.rejects(remote.static("datasets", "nothing"), /no static method datasets\.nothing/);
    await assert.rejects(remote.static("Window", "open"), /no static method Window\.open/);
  } finally {
    await remote.terminate();
  }
});
