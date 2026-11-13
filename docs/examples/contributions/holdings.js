import { datasets } from "finalytics";

const performance = datasets.portfolio().performance();
let total = 0;
for (const item of performance.contributions()) {
  console.log(item.holding, item.contribution.toFixed(6));
  total += item.contribution;
}
console.log("Holdings total:", total.toFixed(6));
console.log("Portfolio return:", performance.timeWeighted().toFixed(6));
