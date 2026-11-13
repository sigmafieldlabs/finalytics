import { datasets } from "finalytics";

const weights = datasets.portfolio().weights();
for (const name of Object.keys(weights).sort()) {
  console.log(name, weights[name].toFixed(6));
}
console.log("Total:", Object.values(weights)
  .reduce((a, b) => a + b, 0).toFixed(6));
