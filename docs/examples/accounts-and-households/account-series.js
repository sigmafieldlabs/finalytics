import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const series = portfolio.accountSeries();
console.log("Dates:", series.dates.length);
for (const line of series.lines) {
  console.log(line.account, line.values.at(-1), line.flows.at(-1));
}
console.log("Household:", portfolio.valueSeries().values.at(-1));
