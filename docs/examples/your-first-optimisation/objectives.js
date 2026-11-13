import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
for (const options of [
  { method: "minimum_variance" },
  { method: "target_return", target: 0.06 },
  { method: "target_volatility", target: 0.08 },
]) {
  const result = assets.optimise(options);
  console.log(options.method, result.results.expected_return.toFixed(6),
    result.results.volatility.toFixed(6));
}
