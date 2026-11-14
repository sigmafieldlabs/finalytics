import { createEngine } from "finalytics/client.js";

const worker = new Worker(
  new URL("finalytics/worker.js", import.meta.url),
  { type: "module" },
);
const engine = createEngine(worker);
try {
  const prices = await engine.static("datasets", "assetClassPrices");
  const assets = await engine.assets(prices, {
    risk_free_asset: "T-Bills",
  });
  const result = await assets.call("optimise", {
    method: "minimum_variance",
    max_weight: 0.4,
  });
  const text = await engine.call("reportText", result.report);
  console.log(text);
  await assets.free();
} finally {
  await engine.terminate();
}
