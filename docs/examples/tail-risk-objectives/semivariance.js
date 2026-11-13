import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "minimum_semivariance",
  max_weight: 0.5,
});

console.log(`${result}`);
result.show();
