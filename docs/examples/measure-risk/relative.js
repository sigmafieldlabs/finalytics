import { relativeStatistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const benchmark = [
    0.015, -0.010, 0.025, 0.010, -0.030, 0.012,
    0.020, -0.005, 0.010, 0.022, -0.018, 0.011,
];
const stats = relativeStatistics(returns, benchmark, {
  periods_per_year: 12,
  risk_free_rate: 0.03,
});
for (const name of [
  "active_return", "tracking_error", "information_ratio", "beta",
  "alpha", "correlation", "downside_beta", "upside_beta",
  "downside_capture", "upside_capture",
]) {
  const label = name
    .replaceAll("_", " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
  console.log(`${label}: ${stats[name].toFixed(6)}`);
}
