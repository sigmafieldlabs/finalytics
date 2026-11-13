import { factorModel, solve } from "finalytics";

const model = factorModel({
  assets: ["equity", "credit", "bonds"],
  exposures: [[1, 0], [0.8, 1], [0, 0.5]],
  factor_covariance: {
    labels: ["market", "spread"],
    values: [0.04, 0, 0, 0.01],
  },
  specific_variances: [0.0025, 0.0025, 0.001],
});
const solution = solve(
  {
    labels: model.assets,
    factor_model: model,
    constraints: {
      factor_exposures: [
        { factor: "market", min: 0.4, max: 0.6 },
      ],
    },
  },
  { type: "minimum_variance" },
);
console.log(`Status: ${solution.status}`);
solution.weights.forEach((weight, index) => {
  console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
});
const exposure = solution.weights[0] + 0.8 * solution.weights[1];
console.log(`Market exposure: ${exposure.toFixed(6)}`);
