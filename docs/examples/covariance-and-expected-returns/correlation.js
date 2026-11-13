import { correlation } from "finalytics";

const covariance = {
  labels: ["equity", "credit", "bonds"],
  values: [
    0.0400, 0.0100, 0.0000,
    0.0100, 0.0900, 0.0000,
    0.0000, 0.0000, 0.0025,
  ],
};
const result = correlation(covariance);
result.labels.forEach((label, row) => {
  const values = result.values.slice(row * 3, row * 3 + 3);
  console.log(label, ...values.map((value) => value.toFixed(6)));
});
