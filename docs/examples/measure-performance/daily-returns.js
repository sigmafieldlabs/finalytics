import { Performance } from "finalytics";

const performance = Performance.fromValues(
  ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"],
  [1000.0, 1050.0, 1600.0, 1580.0],
  [0.0, 0.0, 500.0, 0.0],
);
performance.returns.forEach((value, k) => {
  console.log(performance.dates[k + 1], value.toFixed(6));
});
