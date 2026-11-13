import { Assets, datasets } from "finalytics";

const methods = [
  "historical",
  "james_stein",
  { type: "ewma", decay: 0.97 },
];
for (const method of methods) {
  const assets = new Assets(datasets.assetClassPrices(), {
    risk_free_asset: "T-Bills",
    mean_returns: method,
  });
  console.log(typeof method === "string" ? method : "ewma");
  for (const [label, value] of Object.entries(assets.meanReturns())) {
    console.log(`  ${label}: ${value.toFixed(6)}`);
  }
}
