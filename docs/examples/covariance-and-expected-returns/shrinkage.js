import { covariance } from "finalytics";

const returns = {
  labels: ["equity", "credit", "bonds"],
  columns: [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
  ],
};
for (const type of ["ledoit_wolf", "oas", "constant_correlation"]) {
  const estimate = covariance(returns, { type });
  console.log(
    `${type}: shrinkage=${estimate.shrinkage.toFixed(6)} ` +
    `equity_variance=${estimate.matrix.values[0].toFixed(8)}`,
  );
}
