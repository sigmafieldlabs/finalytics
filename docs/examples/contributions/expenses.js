import { datasets } from "finalytics";

const performance = datasets.portfolio().performance();
const contributions = performance.contributions();
const total = contributions.reduce((sum, c) => sum + c.contribution, 0) +
  performance.expenses();
console.log("Expenses:", performance.expenses().toFixed(6));
console.log("With expenses:", total.toFixed(6));
console.log("Return:", performance.timeWeighted().toFixed(6));
