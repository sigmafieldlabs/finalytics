import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.optimise({ method: "minimum_variance" });
const weights = Object.entries(result.results.weights)
  .sort(([a], [b]) => a.localeCompare(b));
for (const [name, weight] of weights) {
  console.log(name, weight.toFixed(6));
}
console.log("Status:", result.results.status);
