import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "minimum_variance",
  cash: { min: 0, max: 0.2 },
});

console.log(`${result}`);
result.show();
