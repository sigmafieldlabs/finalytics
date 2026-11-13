import { Performance, datasets } from "finalytics";

const performance = Performance.fromValues(
  ["2023-03-31", "2026-03-31"], [100.0, 133.1],
);
const found = performance.period("3Y");
console.log("Cumulative:", found.cumulative.toFixed(6));
console.log("Annualised:", found.annualised.toFixed(6));
const short = datasets.portfolio().performance().period("1Y");
console.log("One year:", short.cumulative.toFixed(6));
