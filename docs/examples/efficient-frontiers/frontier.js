import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const frontier = assets.frontier(6, { max_weight: 0.5 });
for (const point of frontier) {
  console.log(
    `return=${point.expected_return.toFixed(6)}, ` +
      `volatility=${point.volatility.toFixed(6)}`,
  );
}
