import { Assets, datasets, tailRisk } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
assets.returns.forEach((column, index) => {
  const result = tailRisk(column, 0.95, "cornish_fisher");
  console.log(
    `${assets.labels[index]}: VaR=${result.value_at_risk.toFixed(6)} ` +
    `ES=${result.expected_shortfall.toFixed(6)} ` +
    `clamped=${result.shortfall_at_var}`,
  );
});
