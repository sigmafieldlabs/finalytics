import { Assets } from "finalytics";

const assets = Assets.fromCsv(
  "date,VOO,AGG\n" +
  "2026-01-30,500,98\n" +
  "2026-02-27,505,98.2\n" +
  "2026-03-31,510,99.1\n",
);
console.log("Labels:", assets.labels.join(", "));
console.log("Periods per year:", assets.periodsPerYear);
