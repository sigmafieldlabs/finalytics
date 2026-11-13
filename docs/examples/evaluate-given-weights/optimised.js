import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const optimised = assets.optimise({
  method: "maximum_sharpe",
  max_weight: 0.4,
});
const result = assets.evaluate({
  weights: optimised.results.weights,
  name: "Optimised weights",
});

console.log(`${result}`);
result.show();
