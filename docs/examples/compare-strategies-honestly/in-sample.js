import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.compare({
  methods: ["equal_weight", "minimum_variance", "maximum_sharpe"],
  max_weight: 0.4,
  name: "In-sample illustration",
});

console.log(`${result}`);
result.show();
