import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
const weights = {
  "US Equity": 1.2,
  "Intl Equity": -0.3,
  Bonds: 0.3,
  Gold: -0.1,
  "Real Estate": -0.1,
};
const result = assets.evaluate({
  weights,
  name: "Long short evaluation",
});

console.log(`${result}`);
result.show();
