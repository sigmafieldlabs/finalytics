import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.plan({
  end: "2040-12",
  start_value: 10000,
  flows: [{ type: "saving", amount: 500, every: "month" }],
  weights: { "US Equity": 1 },
  inflation: 0.02,
  model: { type: "bootstrap", block: 6 },
  simulations: 200,
});

console.log(`${result}`);
result.show();
