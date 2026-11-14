import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.report({
  benchmark: "BENCH",
  group_by: ["asset_class", "region"],
  confidences: [0.9, 0.95, 0.99],
  rolling_window: 63,
  histogram_bins: 20,
  top: 5,
  name: "Risk presentation options",
});

console.log(`${result}`);
result.show();
