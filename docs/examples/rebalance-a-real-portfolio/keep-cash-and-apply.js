import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const target = portfolio.assets().optimise({
  method: "minimum_variance",
  bounds: { min: 0.05, max: 0.4 },
});
const result = portfolio.rebalance(target, {
  keep_cash: true,
  whole_units: true,
});
console.log(`${result}`);
result.show();

const after = portfolio.withRebalance(result).report();
console.log(`${after}`);
after.show();
