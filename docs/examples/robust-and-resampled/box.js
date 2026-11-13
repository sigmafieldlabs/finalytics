import { solve } from "finalytics";

const labels = ["us equity", "intl equity", "bonds"];
const covariance = {
  labels,
  values: [
    0.0256, 0.02304, 0.0008,
    0.02304, 0.0324, 0.0009,
    0.0008, 0.0009, 0.0025,
  ],
};
const solution = solve(
  { labels, covariance, expected_returns: [0.07, 0.08, 0.03] },
  {
    type: "robust_mean_variance",
    risk_aversion: 4,
    uncertainty: { type: "box", half_widths: [0.01, 0.015, 0.005] },
  },
);
console.log(`Status: ${solution.status}`);
solution.weights.forEach((weight, index) => {
  console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
});
console.log(`Objective: ${solution.objective_value.toFixed(6)}`);
