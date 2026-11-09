// The contracts between features: each verb takes what the others give it, in every shape of
// data: the sample household's assets (five funds, euro cash, and dollar cash as the risk-free
// asset), the asset classes with T-bills as the risk-free asset, and the asset classes without
// them at a 2% rate.
import assert from "node:assert/strict";
import { test } from "node:test";

import { engine } from "./setup.mjs";

const fl = await engine();

const portfolio = fl.datasets.portfolio();
const household = portfolio.assets();
const prices = fl.datasets.assetClassPrices();
const bills = new fl.Assets(prices, { risk_free_asset: "T-Bills" });
const keep = prices.labels.map((l, i) => [l, i]).filter(([l]) => l !== "T-Bills");
const rate = new fl.Assets(
  { dates: prices.dates, labels: keep.map(([l]) => l), columns: keep.map(([, i]) => prices.columns[i]) },
  { risk_free_rate: 0.02 },
);
const shapes = { household, bills, rate };

test("minimum CVaR with cash reports the shortfall of every weight", () => {
  for (const [shape, assets] of Object.entries(shapes)) {
    const result = assets.optimise({ method: "minimum_cvar", cash: { min: 0, max: 0.2 } });
    const cash = assets.riskFreeAsset ?? "Cash";
    assert.ok(cash in result.results.weights, shape);
    assert.ok(result.report.summary.expected_shortfall, shape);
    const shares = result.report.allocation.reduce((s, a) => s + a.risk_share, 0);
    assert.ok(Math.abs(shares - 1) < 1e-6, `${shape}: ${shares}`);
    assert.ok(result.report.allocation.some((a) => a.asset === cash), shape);
  }
});
