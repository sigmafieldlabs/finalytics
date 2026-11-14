import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.plan({
  goal: { type: "target", amount: 150000, by: "2030-06" },
  name: "House deposit",
  start_value: 60000,
  flows: [
    { type: "saving", amount: 2500, every: "month" },
    { type: "spending", name: "Car", amount: 25000, on: "2028-03" },
  ],
  weights: { "US Equity": 0.6, Bonds: 0.4 },
  inflation: 0.025,
  fee: 0.0025,
  simulations: 1000,
  currency: "USD",
});

console.log(`${result}`);
result.show();
