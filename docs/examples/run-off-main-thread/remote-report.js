import { createEngine } from "finalytics/client.js";

const worker = new Worker(
  new URL("finalytics/worker.js", import.meta.url),
  { type: "module" },
);
const engine = createEngine(worker);
try {
  const file = (name) => engine.static(
    "datasets", "file", `portfolio/${name}.csv`,
  );
  const portfolio = await engine.portfolioFromCsv({
    transactions: await file("transactions"),
    instruments: await file("instruments"),
    accounts: await file("accounts"),
    prices: await file("prices"),
    fx: await file("fx"),
  });
  const report = await portfolio.call("report", { benchmark: "BENCH" });
  const text = await engine.call("reportText", report);
  const html = await engine.call("reportHtml", report, null, "auto");
  console.log(text);
  console.log(`HTML report ready (${html.length} characters)`);
  await portfolio.free();
} finally {
  await engine.terminate();
}
