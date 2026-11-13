import { trackingErrorDecomposition } from "finalytics";

const covariance = {
  labels: ["equity", "credit", "bonds"],
  values: [
    0.0400, 0.0100, -0.0060,
    0.0100, 0.0900, -0.0090,
    -0.0060, -0.0090, 0.0100,
  ],
};
const result = trackingErrorDecomposition(
  [0.6, 0.2, 0.2],
  [0.4, 0.4, 0.2],
  covariance,
);
console.log(`Tracking error: ${result.volatility.toFixed(6)}`);
for (const item of result.contributions) {
  console.log(
    `${item.label}: active_weight=${item.weight.toFixed(6)} ` +
    `component=${item.component.toFixed(6)} share=${item.share.toFixed(6)}`,
  );
}
