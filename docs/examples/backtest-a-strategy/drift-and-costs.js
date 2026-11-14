import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.backtest({
  weights: { "US Equity": 0.6, Bonds: 0.4 },
  name: "Balanced",
  rebalance: { drift: { band: 0.05 } },
  trading_cost_bps: 25,
});

console.log(`${result}`);
result.show();
