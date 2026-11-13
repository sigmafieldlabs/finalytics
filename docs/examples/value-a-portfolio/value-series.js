import { datasets } from "finalytics";

const series = datasets.portfolio().valueSeries();
console.log("Dates:", series.dates.length);
for (let k = series.dates.length - 3; k < series.dates.length; k++) {
  console.log(series.dates[k], series.values[k], series.flows[k]);
}
