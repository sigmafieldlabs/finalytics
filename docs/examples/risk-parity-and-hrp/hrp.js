import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "hierarchical_risk_parity",
  linkage: "ward",
});

console.log(`${result}`);
result.show();
