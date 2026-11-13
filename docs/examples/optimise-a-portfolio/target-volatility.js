import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "target_volatility",
  target: 0.1,
  max_weight: 0.4,
});

console.log(`${result}`);
