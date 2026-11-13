import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "maximum_sharpe",
  benchmark_weights: { "US Equity": 0.6, Bonds: 0.4 },
  max_tracking_error: 0.08,
});

console.log(`${result}`);
result.show();
