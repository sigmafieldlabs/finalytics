import { Performance } from "finalytics";

const performance = Performance.fromValues(
  ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"],
  [1000.0, 1050.0, 1600.0, 1580.0],
  [0.0, 0.0, 500.0, 0.0],
);
console.log("Time-weighted:", performance.timeWeighted().toFixed(6));
console.log("Money-weighted:", performance.moneyWeighted().toFixed(6));
console.log("Modified Dietz:", performance.modifiedDietz().toFixed(6));
