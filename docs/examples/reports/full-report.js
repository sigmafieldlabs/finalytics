import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.report({
  ...datasets.reportOptions(),
  name: "Morgan family portfolio",
});

console.log(`${result}`);
result.show();
