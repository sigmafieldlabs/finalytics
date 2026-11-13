import { Performance } from "finalytics";

const performance = Performance.fromValues(
  ["2025-12-31", "2026-03-31"], [1000.0, 1070.323],
);
const excess = performance.excessReturn(
  ["2025-12-31", "2026-03-31"], [100.0, 104.0], "ITD",
);
console.log("Portfolio:", excess.portfolio.toFixed(6));
console.log("Benchmark:", excess.benchmark.toFixed(6));
console.log("Arithmetic:", excess.arithmetic.toFixed(6));
console.log("Geometric:", excess.geometric.toFixed(6));
