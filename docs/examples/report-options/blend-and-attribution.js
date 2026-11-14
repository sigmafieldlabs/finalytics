import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.report({
  benchmark: { VTI: 0.55, BND: 0.35, GLD: 0.1 },
  attribution: { scheme: "asset_class" },
  group_by: ["asset_class"],
  name: "Against a blended benchmark",
});

console.log(`${result}`);
result.show();
