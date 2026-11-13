import { relativeStatistics } from "finalytics";

const returns = [0.020, -0.010, 0.015, 0.005, -0.004, 0.012];
const benchmark = [0.015, -0.008, 0.010, 0.006, -0.003, 0.009];
const found = relativeStatistics(returns, benchmark, { periods_per_year: 12 });
console.log("Active return:", found.active_return.toFixed(6));
console.log("Tracking error:", found.tracking_error.toFixed(6));
console.log("Information ratio:", found.information_ratio.toFixed(6));
