import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const path = {
  type: "glide_path",
  points: {
    "2026-11": { "US Equity": 0.8, Bonds: 0.2 },
    "2030-01": { "US Equity": 0.3, Bonds: 0.7 },
  },
};
const result = assets.plan({
  goal: { type: "target", amount: 150000, by: "2030-06" },
  start_value: 60000,
  flows: [{ type: "saving", amount: 2500, every: "month" }],
  weights: path,
  inflation: 0.025,
  model: "normal",
  simulations: 500,
  seed: 7,
});

console.log(`${result}`);
result.show();
