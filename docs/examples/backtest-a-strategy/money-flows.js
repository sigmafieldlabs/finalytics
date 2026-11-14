import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.backtest({
  weights: { "US Equity": 0.6, Bonds: 0.4 },
  start_value: 10000,
  flows: [
    { date: "2020-01-31", amount: 1000 },
    { date: "2021-06-15", amount: -500 },
  ],
  rebalance: "yearly",
});

console.log(`${result}`);
result.show();
