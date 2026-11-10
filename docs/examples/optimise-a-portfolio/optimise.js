import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({ method: "maximum_sharpe", max_weight: 0.4 });

console.log(`${result}`);
