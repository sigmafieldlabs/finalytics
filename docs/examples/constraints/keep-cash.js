import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const current = portfolio.weights();
const result = portfolio.assets().optimise({
  method: "maximum_sharpe",
  current_weights: current,
  max_turnover: 0.15,
  keep_cash: true,
});

console.log(`${result}`);
result.show();
