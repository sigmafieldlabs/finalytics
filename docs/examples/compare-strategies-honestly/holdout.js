import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.compare({
  methods: ["minimum_variance", "maximum_sharpe"],
  weights: { "60/40": { "US Equity": 0.6, Bonds: 0.4 } },
  validation: { holdout: { start: "2016-11-01" } },
  rebalance: "quarterly",
  trading_cost_bps: 10,
  max_weight: 0.6,
});

console.log(`${result}`);
result.show();
