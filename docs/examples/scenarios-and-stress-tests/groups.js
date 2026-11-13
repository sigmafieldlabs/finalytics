import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.shockScenario(
  {
    groups: [
      { scheme: "asset_class", value: "Equity", shock: -0.25 },
      { scheme: "asset_class", value: "Real assets", shock: -0.10 },
      { scheme: "asset_class", value: "Fixed income", shock: 0.03 },
    ],
  },
  { group_by: ["asset_class"] },
);
console.log(`Value before: ${result.value_before.toFixed(2)}`);
console.log(`Value after: ${result.value_after.toFixed(2)}`);
console.log(`Portfolio return: ${result.portfolio_return.toFixed(6)}`);
for (const group of result.groups[0].groups) {
  console.log(`${group.group}: contribution=${group.contribution.toFixed(6)}`);
}
