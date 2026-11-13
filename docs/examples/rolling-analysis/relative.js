import { rollingRelativeStatistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const benchmark = [
    0.015, -0.010, 0.025, 0.010, -0.030, 0.012,
    0.020, -0.005, 0.010, 0.022, -0.018, 0.011,
];
const windows = rollingRelativeStatistics(returns, benchmark, 6, {
  periods_per_year: 12,
});
windows.forEach((stats, index) => {
  console.log(
    `Window ${index}: active=${stats.active_return.toFixed(6)} ` +
    `TE=${stats.tracking_error.toFixed(6)} beta=${stats.beta.toFixed(6)} ` +
    `correlation=${stats.correlation.toFixed(6)}`,
  );
});
