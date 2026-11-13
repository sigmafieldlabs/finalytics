import { datasets } from "finalytics";

const valuation = datasets.portfolio().valuation();
console.log("Date:", valuation.date);
console.log("Positions:", valuation.positions.length);
console.log("Cash balances:", valuation.cash.length);
console.log("Total:", valuation.total_base);
