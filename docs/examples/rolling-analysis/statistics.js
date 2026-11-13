import { rollingStatistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const windows = rollingStatistics(returns, 6, {
  periods_per_year: 12,
  risk_free_rate: 0.03,
});
console.log(`Windows: ${windows.length}`);
for (const index of [0, windows.length - 1]) {
  const stats = windows[index];
  console.log(
    `Window ${index}: return=${stats.annualised_return.toFixed(6)} ` +
    `volatility=${stats.volatility.toFixed(6)} ` +
    `Sharpe=${stats.sharpe_ratio.toFixed(6)}`,
  );
}
