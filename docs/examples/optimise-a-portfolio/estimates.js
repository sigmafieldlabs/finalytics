import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "maximum_sharpe",
  expected_returns: { "US Equity": 0.04 },
  max_weight: 0.4,
});

console.log(`${result}`);
result.show();
