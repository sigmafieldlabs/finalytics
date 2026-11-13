import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "target_return",
  target: 0.2,
  max_weight: 0.4,
});

console.log(result.results.status);
