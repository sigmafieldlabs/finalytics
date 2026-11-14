import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.report({
  benchmark: "BENCH",
  accounts: ["brokerage"],
  start: "2026-01-01",
  as_of: "2026-10-30",
  group_by: ["asset_class"],
  name: "Brokerage in 2026",
});

console.log(`${result}`);
result.show();
