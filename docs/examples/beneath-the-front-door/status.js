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
    constraints: { bounds: { min: 0, max: 0.2 } },
  },
  { type: "minimum_variance" },
);
console.log(`Status: ${solution.status}`);
if (solution.weights === null) {
  console.log("Weights: none");
} else {
  solution.weights.forEach((weight, index) => {
    console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
  });
}
