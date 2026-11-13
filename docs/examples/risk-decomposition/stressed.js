import { riskDecomposition, stressedCovariance } from "finalytics";

const covariance = {
  labels: ["equity", "bonds"],
  values: [0.04, 0.01, 0.01, 0.09],
};
const stressed = stressedCovariance(covariance, {
  volatility_scale: 1.5,
  correlation_shift: 0.5,
});
for (const [name, matrix] of [["Before", covariance], ["After", stressed]]) {
  const result = riskDecomposition([0.6, 0.4], matrix);
  console.log(`${name}: volatility=${result.volatility.toFixed(6)}`);
  for (const item of result.contributions) {
    console.log(`  ${item.label}: share=${item.share.toFixed(6)}`);
  }
}
