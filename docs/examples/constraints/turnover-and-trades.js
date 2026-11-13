import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const current = {
  "US Equity": 0.40,
  "Intl Equity": 0.10,
  Bonds: 0.30,
  Gold: 0.10,
  "Real Estate": 0.05,
  "T-Bills": 0.05,
};
const result = assets.optimise({
  method: "maximum_sharpe",
  current_weights: current,
  max_turnover: 0.15,
  min_trade: 0.02,
  max_trades: 3,
});

console.log(`${result}`);
result.show();
