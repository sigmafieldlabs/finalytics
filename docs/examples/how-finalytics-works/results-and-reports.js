import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const result = assets.optimise({ method: "minimum_variance" });

console.log("Expected return:", result.results.expected_return.toFixed(6));
console.log("Volatility:", result.results.volatility.toFixed(6));
console.log(`${result}`.replace(/\n$/, ""));
result.show();
