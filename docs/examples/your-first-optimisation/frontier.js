import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
for (const point of assets.frontier(5, { max_weight: 0.50 })) {
  console.log(point.expected_return.toFixed(6), point.volatility.toFixed(6));
}
