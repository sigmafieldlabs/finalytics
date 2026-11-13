import { covariance } from "finalytics";

const returns = {
  labels: ["equity", "credit", "bonds"],
  columns: [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
  ],
};
const weighted = covariance(returns, { type: "ewma", decay: 0.90 });
const statistical = covariance(returns, { type: "statistical", factors: 2 });
console.log(`EWMA equity variance: ${weighted.matrix.values[0].toFixed(8)}`);
console.log(`EWMA credit variance: ${weighted.matrix.values[4].toFixed(8)}`);
console.log(
  `Statistical equity variance: ${statistical.matrix.values[0].toFixed(8)}`,
);
console.log(
  `Statistical bond variance: ${statistical.matrix.values[8].toFixed(8)}`,
);
