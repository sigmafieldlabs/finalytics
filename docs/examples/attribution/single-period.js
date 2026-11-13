import { brinsonFachler } from "finalytics";

const segments = [
  ["UK", 0.4, 0.20, 0.4, 0.10],
  ["Japan", 0.3, -0.05, 0.2, -0.04],
  ["US", 0.3, 0.06, 0.4, 0.08],
].map(([name, portfolio_weight, portfolio_return,
  benchmark_weight, benchmark_return]) => ({
  name, portfolio_weight, portfolio_return,
  benchmark_weight, benchmark_return,
}));
const result = brinsonFachler([{
  start: "2026-01-01", end: "2026-12-31", segments,
}]);
console.log("Excess:", result.excess_return.toFixed(6));
for (const segment of result.segments) {
  console.log(segment.name, segment.allocation.toFixed(6),
    segment.selection.toFixed(6), segment.interaction.toFixed(6));
}
