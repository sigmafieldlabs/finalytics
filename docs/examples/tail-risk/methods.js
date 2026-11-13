import { tailRisk } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
for (const method of ["historical", "parametric", "cornish_fisher"]) {
  const result = tailRisk(returns, 0.90, method);
  console.log(
    `${method}: VaR=${result.value_at_risk.toFixed(6)} ` +
    `ES=${result.expected_shortfall.toFixed(6)}`,
  );
}
