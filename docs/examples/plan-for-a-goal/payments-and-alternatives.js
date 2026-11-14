import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.plan({
  goal: {
    type: "payments",
    amount: 4000,
    start: "2046-11",
    end: "2066-10",
  },
  name: "Retirement income",
  start_value: 250000,
  flows: [{ type: "saving", amount: 1500, every: "month" }],
  weights: {
    type: "fixed",
    name: "Balanced 60/40",
    weights: { "US Equity": 0.6, Bonds: 0.4 },
  },
  alternatives: {
    "Cautious 30/70": { "US Equity": 0.3, Bonds: 0.7 },
    "Growth 80/20": { "US Equity": 0.8, Bonds: 0.2 },
  },
  inflation: 0.025,
  chance: [0.85, 0.9],
  simulations: 500,
});

console.log(`${result}`);
result.show();
