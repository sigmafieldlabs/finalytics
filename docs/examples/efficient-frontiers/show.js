import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.evaluate({
  weights: { "US Equity": 0.6, Bonds: 0.4 },
  max_weight: 0.5,
  frontier: true,
  name: "Sixty forty",
});

console.log(`${result}`);
result.show();
