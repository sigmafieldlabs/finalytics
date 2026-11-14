import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const target = portfolio.assets().optimise({
  method: "minimum_variance",
  bounds: { min: 0.05, max: 0.4 },
});
const result = portfolio.rebalance(target, {
  whole_units: true,
  fee_bps: 5,
  minimum_fee: 1,
  tax: [0.37, 0.2],
});

console.log(`${result}`);
result.show();
