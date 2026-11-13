import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "maximum_sharpe",
  bounds: { min: 0, max: 0.4 },
  asset_bounds: { Gold: { min: 0, max: 0.1 } },
  groups: {
    Equity: {
      assets: ["US Equity", "Intl Equity"],
      min: 0.3,
      max: 0.6,
    },
  },
});

console.log(`${result}`);
result.show();
