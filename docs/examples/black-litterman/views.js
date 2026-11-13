import { blackLitterman } from "finalytics";

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
  views: [
    {
      portfolio: { "intl equity": 1, "us equity": -1 },
      expected_return: 0.02,
      uncertainty: { type: "confidence", confidence: 0.6 },
    },
    {
      portfolio: { bonds: 1 },
      expected_return: 0.04,
      uncertainty: { type: "confidence", confidence: 0.7 },
    },
  ],
});
estimate.labels.forEach((label, index) => {
  console.log(
    `${label}: prior=${estimate.prior_returns[index].toFixed(6)}, ` +
      `posterior=${estimate.posterior_returns[index].toFixed(6)}`,
  );
});
estimate.view_variances.forEach((variance, index) => {
  console.log(`View ${index + 1} variance: ${variance.toFixed(8)}`);
});
