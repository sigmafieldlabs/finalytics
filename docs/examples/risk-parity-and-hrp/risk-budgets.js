import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const result = assets.optimise({
  method: "risk_budgeting",
  budgets: {
    "US Equity": 0.30,
    "Intl Equity": 0.20,
    Bonds: 0.20,
    Gold: 0.15,
    "Real Estate": 0.15,
  },
});

console.log(`${result}`);
result.show();
