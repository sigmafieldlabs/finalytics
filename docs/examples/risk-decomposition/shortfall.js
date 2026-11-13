import { shortfallDecomposition } from "finalytics";

const scenarios = {
  labels: ["equity", "bonds"],
  columns: [
    [-0.08, 0.02, 0.03, -0.01],
    [0.01, 0.00, 0.005, -0.004],
  ],
};
const result = shortfallDecomposition([0.6, 0.4], scenarios, 0.75);
console.log(`Expected shortfall: ${result.expected_shortfall.toFixed(6)}`);
for (const item of result.contributions) {
  console.log(
    `${item.label}: marginal=${item.marginal.toFixed(6)} ` +
    `component=${item.component.toFixed(6)} share=${item.share.toFixed(6)}`,
  );
}
