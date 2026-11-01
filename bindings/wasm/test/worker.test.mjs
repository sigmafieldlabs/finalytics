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

    const books = await remote.books(portfolio, market.toDocument());
    const perf = await books.call("performance");
    const local = new fl.Books(portfolio, market).performance();
    assert.deepEqual(await perf.get("returns"), local.returns);
    assert.equal(await perf.call("timeWeighted"), local.timeWeighted());
    await perf.free();
    await books.free();

    await assert.rejects(remote.call("statistics", [0.01]), { name: "ValidationError" });
  } finally {
    await remote.terminate();
  }
});
