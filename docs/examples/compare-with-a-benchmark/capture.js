import { relativeStatistics } from "finalytics";

const returns = [0.020, -0.010, 0.015, 0.005, -0.004, 0.012];
const benchmark = [0.015, -0.008, 0.010, 0.006, -0.003, 0.009];
const found = relativeStatistics(returns, benchmark, { periods_per_year: 12 });
console.log("Upside capture:", found.upside_capture.toFixed(6));
console.log("Downside capture:", found.downside_capture.toFixed(6));
console.log("Beta:", found.beta.toFixed(6));
