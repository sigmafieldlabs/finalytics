import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const options = datasets.reportOptions();
const result = portfolio.report({
  ...options,
  start: "2025-01-01",
  name: "Attribution and scenarios since 2025",
});

console.log(`${result}`);
result.show();
