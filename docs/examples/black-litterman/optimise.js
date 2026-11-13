import { blackLitterman, solve } from "finalytics";

const labels = ["us equity", "intl equity", "bonds"];
const estimate = blackLitterman({
  covariance: {
    labels,
    values: [
      0.0256, 0.02304, 0.0008,
      0.02304, 0.0324, 0.0009,
      0.0008, 0.0009, 0.0025,
    ],
  },
  prior: {
    type: "market",
    weights: [0.55, 0.25, 0.20],
    risk_aversion: 2.5,
  },
  views: [{
    portfolio: { "intl equity": 1, "us equity": -1 },
    expected_return: 0.02,
    uncertainty: { type: "confidence", confidence: 0.6 },
  }],
});
const solution = solve(
  {
    labels: estimate.labels,
    covariance: estimate.posterior_covariance,
    expected_returns: estimate.posterior_returns,
  },
  { type: "mean_variance", risk_aversion: 2.5 },
);
console.log(`Status: ${solution.status}`);
solution.weights.forEach((weight, index) => {
  console.log(`${solution.labels[index]}: ${weight.toFixed(6)}`);
});
