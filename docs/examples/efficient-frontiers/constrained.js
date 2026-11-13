import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const frontier = assets.frontier(5, {
  bounds: { min: 0, max: 0.5 },
  groups: {
    Equity: {
      assets: ["US Equity", "Intl Equity"],
      min: 0.2,
      max: 0.7,
    },
  },
  cash: { min: 0, max: 0.2 },
});
for (const point of frontier) {
  console.log(
    `return=${point.expected_return.toFixed(6)}, ` +
      `volatility=${point.volatility.toFixed(6)}`,
  );
  for (const asset of Object.keys(point.weights).sort()) {
    console.log(`  ${asset}: ${point.weights[asset].toFixed(6)}`);
  }
}
