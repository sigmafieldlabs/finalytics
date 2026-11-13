import { riskDecomposition } from "finalytics";

const covariance = {
  labels: ["equity", "credit", "bonds"],
  values: [
    0.0400, 0.0100, -0.0060,
    0.0100, 0.0900, -0.0090,
    -0.0060, -0.0090, 0.0100,
  ],
};
const result = riskDecomposition([0.5, 0.3, 0.2], covariance);
console.log(`Volatility: ${result.volatility.toFixed(6)}`);
for (const item of result.contributions) {
  console.log(
    `${item.label}: marginal=${item.marginal.toFixed(6)} ` +
    `component=${item.component.toFixed(6)} share=${item.share.toFixed(6)}`,
  );
}
const total = result.contributions.reduce(
  (sum, item) => sum + item.component,
  0,
);
const shares = result.contributions.reduce(
  (sum, item) => sum + item.share,
  0,
);
console.log(`Component sum: ${total.toFixed(6)}`);
console.log(`Share sum: ${shares.toFixed(6)}`);
