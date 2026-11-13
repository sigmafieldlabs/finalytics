import { impliedRiskAversion } from "finalytics";

const marketReturns = [
  0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
  0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const estimate = impliedRiskAversion(marketReturns, {
  periods_per_year: 12,
  risk_free_rate: 0.02,
});
console.log(`Implied risk aversion: ${estimate.toFixed(6)}`);
