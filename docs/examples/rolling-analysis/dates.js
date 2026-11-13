import { rollingStatistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const dates = [
  "2026-01-30", "2026-02-27", "2026-03-31", "2026-04-30",
  "2026-05-29", "2026-06-30", "2026-07-31", "2026-08-31",
  "2026-09-30", "2026-10-30", "2026-11-30", "2026-12-31",
];
const window = 6;
const windows = rollingStatistics(returns, window, { periods_per_year: 12 });
windows.slice(-3).forEach((stats, offset) => {
  const end = dates[window - 1 + windows.length - 3 + offset];
  console.log(`Ending ${end}: volatility=${stats.volatility.toFixed(6)}`);
});
