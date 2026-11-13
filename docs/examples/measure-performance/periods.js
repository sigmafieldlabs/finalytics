import { datasets } from "finalytics";

const performance = datasets.portfolio().performance();
for (const name of ["MTD", "QTD", "YTD", "1Y", "ITD"]) {
  const found = performance.period(name);
  if (!found) console.log(name, "insufficient history");
  else console.log(name, found.start, found.cumulative.toFixed(6));
}
