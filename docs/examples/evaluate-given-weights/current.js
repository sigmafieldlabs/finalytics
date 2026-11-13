import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.assets().evaluate({
  weights: portfolio.weights(),
  name: "Current household",
});

console.log(`${result}`);
result.show();
