import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.rebalance(
  { VTI: 0.4, EXSA: 0.1, BND: 0.35, GLD: 0.1, VNQ: 0.05 },
  { whole_units: true },
);

console.log(`${result}`);
result.show();
