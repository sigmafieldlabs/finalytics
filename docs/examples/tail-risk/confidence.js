import { tailRisk } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
for (const confidence of [0.90, 0.95, 0.99]) {
  const result = tailRisk(returns, confidence, "historical");
  const level = confidence.toFixed(2);
  const varLoss = result.value_at_risk.toFixed(6);
  console.log(
    `Confidence ${level}: VaR=${varLoss} ` +
    `ES=${result.expected_shortfall.toFixed(6)}`,
  );
}
