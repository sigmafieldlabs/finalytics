import { covariance } from "finalytics";

const returns = {
  labels: ["equity", "credit", "bonds"],
  columns: [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
  ],
};
const estimate = covariance(returns, { type: "sample" });
console.log(`Observations: ${estimate.observations}`);
estimate.matrix.labels.forEach((label, row) => {
  const values = estimate.matrix.values.slice(row * 3, row * 3 + 3);
  console.log(label, ...values.map((value) => value.toFixed(8)));
});
const annual = estimate.matrix.values.map((value) => value * 252);
console.log(`Annual equity variance: ${annual[0].toFixed(8)}`);
