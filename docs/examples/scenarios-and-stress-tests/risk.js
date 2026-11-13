import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const covariance = {
  labels: ["VTI", "EXSA", "BND", "GLD", "VNQ", "EUR cash"],
  values: [
    0.0400, 0, 0, 0, 0, 0,
    0, 0.0625, 0, 0, 0, 0,
    0, 0, 0.0036, 0, 0, 0,
    0, 0, 0, 0.0225, 0, 0,
    0, 0, 0, 0, 0.0400, 0,
    0, 0, 0, 0, 0, 0.0001,
  ],
};
const result = portfolio.shockScenario(
  {
    groups: [
      { scheme: "asset_class", value: "Equity", shock: -0.20 },
    ],
  },
  {
    risk: {
      covariance,
      confidence: 0.99,
      stress: { volatility_scale: 1.5, correlation_shift: 0.5 },
    },
  },
);
for (const [name, risk] of [
  ["Before", result.risk_before],
  ["After", result.risk_after],
]) {
  console.log(`${name}: volatility=${risk.volatility.toFixed(6)}`);
  console.log(`${name}: value at risk=${risk.value_at_risk_base.toFixed(2)}`);
}
