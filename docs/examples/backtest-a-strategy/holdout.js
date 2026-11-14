import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.backtest({
  method: "maximum_sharpe",
  max_weight: 0.4,
  benchmark: { "US Equity": 0.6, Bonds: 0.4 },
  validation: { holdout: { start: "2016-11-01" } },
  rebalance: "quarterly",
  trading_cost_bps: 10,
});

console.log(`${result}`);
result.show();
