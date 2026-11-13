import { factorModel, factorRisk, factorTrackingError } from "finalytics";

const model = factorModel({
  assets: ["SPY", "SAP", "BOND"],
  exposures: [[1.0, 0.0], [1.2, 0.0], [0.0, -6.0]],
  factor_covariance: {
    labels: ["equity", "rates"],
    values: [0.04, -0.0016, -0.0016, 0.0001],
  },
  specific_variances: [0.01, 0.02, 0.0001],
});
const weights = [0.4, 0.3, 0.3];
const benchmark = [0.3, 0.3, 0.4];
const results = [
  ["Portfolio", factorRisk(weights, model)],
  ["Active", factorTrackingError(weights, benchmark, model)],
];
for (const [name, result] of results) {
  console.log(`${name}: volatility=${result.volatility.toFixed(6)}`);
  for (const item of result.factors) {
    console.log(
      `  ${item.factor}: exposure=${item.exposure.toFixed(6)} ` +
      `component=${item.component.toFixed(6)} ` +
      `share=${item.share.toFixed(6)}`,
    );
  }
  console.log(`  specific: share=${result.specific_share.toFixed(6)}`);
}
