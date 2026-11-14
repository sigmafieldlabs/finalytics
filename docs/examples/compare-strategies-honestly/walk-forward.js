import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.compare({
  methods: ["equal_weight", "minimum_variance", "maximum_sharpe"],
  validation: { walk_forward: { years: 5 } },
  rebalance: "quarterly",
  trading_cost_bps: 10,
  max_weight: 0.4,
});

console.log(`${result}`);
result.show();
