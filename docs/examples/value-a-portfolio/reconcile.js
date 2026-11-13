import { datasets } from "finalytics";

const valuation = datasets.portfolio().valuation();
const positions = valuation.positions.reduce((sum, p) =>
  sum + Number(p.market_value_base), 0);
const cash = valuation.cash.reduce((sum, c) =>
  sum + Number(c.value_base), 0);
console.log("Positions:", positions.toFixed(2));
console.log("Cash:", cash.toFixed(2));
console.log("Reconciles:", Math.abs(positions + cash -
  Number(valuation.total_base)) < 0.000001);
