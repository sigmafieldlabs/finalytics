import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "minimum_variance",
  min_effective_assets: 3.5,
});

console.log(`${result}`);
result.show();
