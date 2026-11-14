import { datasets, reportText } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.holdingsReport({
  group_by: ["asset_class", "region"],
});
console.log(reportText(result));

const source = result.provenance;
console.log(`Report: ${source.report}`);
console.log(`As of: ${source.as_of}`);
console.log(`Base currency: ${source.base_currency}`);
console.log(`Source digest: ${source.source.digest}`);
