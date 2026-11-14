import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.compare({
  methods: ["minimum_variance", "maximum_diversification"],
  validation: { walk_forward: { years: 10, expanding: true } },
  rebalance: "yearly",
  trading_cost_bps: 10,
});

console.log(`${result}`);
result.show();
