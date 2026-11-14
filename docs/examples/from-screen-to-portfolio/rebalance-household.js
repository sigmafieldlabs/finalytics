import { Universe, datasets } from "finalytics";

const portfolio = datasets.portfolio();
const universe = Universe.fromCsv(datasets.file("portfolio/prices.csv"), {
  fields: datasets.file("portfolio/instruments.csv"),
  benchmark: "BENCH",
  risk_free_rate: 0.04,
});
const screen = universe.screen({
  rules: [{ measure: "return_1y", op: ">", value: 0 }],
  sort: { measure: "sharpe_ratio", order: "descending" },
});
const weight = 1 / screen.results.matches.length;
const targets = Object.fromEntries(
  screen.results.matches.map((asset) => [asset, weight]),
);
const result = portfolio.rebalance(targets, {
  whole_units: true,
  fee_bps: 5,
  tax: [0.37, 0.2],
  name: "Rebalance to screened funds",
});

console.log(`${result}`);
result.show();
