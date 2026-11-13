import { factorModel, impliedCovariance } from "finalytics";

const model = factorModel({
  assets: ["SPY", "SAP", "BOND"],
  exposures: [[1.0, 0.0], [1.2, 0.0], [0.0, -6.0]],
  factor_covariance: {
    labels: ["equity", "rates"],
    values: [0.04, -0.0016, -0.0016, 0.0001],
  },
  specific_variances: [0.01, 0.02, 0.0001],
});
console.log(`Assets: ${model.assets.join(", ")}`);
console.log(`Factors: ${model.factor_covariance.labels.join(", ")}`);
const implied = impliedCovariance(model);
implied.labels.forEach((label, row) => {
  const values = implied.values.slice(row * 3, row * 3 + 3);
  console.log(label, ...values.map((value) => value.toFixed(6)));
});
