import { brinsonFachler } from "finalytics";

const rows = [
  ["2026-01-01", "2026-04-01",
   [["Equities", 0.6, 0.08, 0.5, 0.06],
    ["Bonds", 0.4, 0.01, 0.5, 0.015]]],
  ["2026-04-01", "2026-07-01",
   [["Equities", 0.65, -0.05, 0.5, -0.04],
    ["Bonds", 0.35, 0.02, 0.5, 0.02]]],
];
const periods = rows.map(([start, end, segments]) => ({
  start, end,
  segments: segments.map(([name, portfolio_weight, portfolio_return,
    benchmark_weight, benchmark_return]) => ({
    name, portfolio_weight, portfolio_return,
    benchmark_weight, benchmark_return,
  })),
}));
const result = brinsonFachler(periods);
console.log("Excess:", result.excess_return.toFixed(6));
console.log("Effects:", (result.allocation + result.selection +
  result.interaction).toFixed(6));
for (const period of result.periods) {
  console.log(period.start, period.linking_coefficient.toFixed(6));
}
