import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.historicalScenario({
  start: "2025-02-19",
  end: "2025-04-08",
});
console.log(`Value before: ${result.value_before.toFixed(2)}`);
console.log(`Value after: ${result.value_after.toFixed(2)}`);
console.log(`Portfolio return: ${result.portfolio_return.toFixed(6)}`);
console.log(`Path observations: ${result.path.dates.length}`);
console.log(`Path start: ${result.path.values[0].toFixed(2)}`);
console.log(
  `Path end: ${result.path.values[result.path.values.length - 1].toFixed(2)}`,
);
console.log(`Worst drawdown: ${result.path.worst_drawdown.depth.toFixed(6)}`);
console.log(`Trough: ${result.path.worst_drawdown.trough}`);
