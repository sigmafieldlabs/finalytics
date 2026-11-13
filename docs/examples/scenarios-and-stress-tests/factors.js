import { datasets, factorModel } from "finalytics";

const portfolio = datasets.portfolio();
const model = factorModel({
  assets: ["VTI", "EXSA", "BND", "GLD", "VNQ"],
  exposures: [
    [1.0, 0.0],
    [1.1, 0.0],
    [0.0, -6.0],
    [0.3, 0.0],
    [0.8, -1.0],
  ],
  factor_covariance: {
    labels: ["equity", "rates"],
    values: [0.04, -0.0016, -0.0016, 0.0001],
  },
  specific_variances: [0.01, 0.01, 0.0025, 0.01, 0.01],
});
const result = portfolio.shockScenario({
  factors: {
    model,
    shocks: { equity: -0.15 },
    propagate: true,
  },
});
for (const [factor, value] of Object.entries(result.factor_returns)) {
  console.log(`${factor}: ${value.toFixed(6)}`);
}
for (const holding of result.holdings.slice(0, 5)) {
  console.log(`${holding.holding}: local=${holding.return_local.toFixed(6)}`);
}
