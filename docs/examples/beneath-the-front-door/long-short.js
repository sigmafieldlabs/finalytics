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
      bounds: { min: -0.5, max: 1 },
      leverage: 1.5,
    },
  },
  { type: "target_return", target: 0.07 },
);
console.log(`Status: ${solution.status}`);
solution.weights.forEach((weight, index) => {
  console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
});
console.log(`Largest violation: ${solution.max_violation.toFixed(9)}`);
const gross = solution.weights.reduce(
  (total, weight) => total + Math.abs(weight),
  0,
);
console.log(`Gross exposure: ${gross.toFixed(6)}`);
