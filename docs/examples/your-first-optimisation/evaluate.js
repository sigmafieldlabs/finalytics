import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.evaluate({
  weights: { "US Equity": 0.60, Bonds: 0.40 },
});
console.log("Expected return:", result.results.expected_return.toFixed(6));
console.log("Volatility:", result.results.volatility.toFixed(6));
console.log("Sharpe ratio:", result.results.sharpe_ratio.toFixed(6));
