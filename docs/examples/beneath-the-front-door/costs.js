import { solve } from "finalytics";

const labels = ["equity", "credit", "bonds"];
const covariance = {
  labels,
  values: [
    0.0400, 0.0060, 0.0020,
    0.0060, 0.0144, 0.0015,
    0.0020, 0.0015, 0.0036,
  ],
};
const solution = solve(
  {
    labels,
    covariance,
    expected_returns: [0.07, 0.045, 0.03],
    constraints: {
      turnover: {
        current: [0.6, 0.2, 0.2],
        costs: [0.003, 0.003, 0.003],
        quadratic_costs: [0.05, 0.10, 0.20],
      },
    },
  },
  { type: "mean_variance", risk_aversion: 4 },
);
console.log(`Status: ${solution.status}`);
solution.weights.forEach((weight, index) => {
  console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
});
console.log(`Largest violation: ${solution.max_violation.toFixed(9)}`);
