import { rollingTailRisk } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const windows = rollingTailRisk(returns, 6, 0.90, "historical");
windows.forEach((result, index) => {
  console.log(
    `Window ${index}: VaR=${result.value_at_risk.toFixed(6)} ` +
    `ES=${result.expected_shortfall.toFixed(6)}`,
  );
});
