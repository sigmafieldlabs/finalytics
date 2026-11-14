import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.backtest({
  weights: { "US Equity": 0.7, Gold: 0.3 },
  name: "Buy and hold",
  rebalance: "never",
});

console.log(`${result}`);
result.show();
