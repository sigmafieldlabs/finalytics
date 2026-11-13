import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const report = portfolio.report({
  benchmark: "BENCH",
  group_by: ["asset_class", "region"],
});

console.log(`${report}`);
report.show();
