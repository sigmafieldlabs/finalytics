import { Assets, Universe, datasets } from "finalytics";

const csv = datasets.file("portfolio/prices.csv");
const universe = Universe.fromCsv(csv, {
  benchmark: "BENCH",
  risk_free_rate: 0.04,
});
const screen = universe.screen({
  rules: [{ measure: "return_1y", op: ">", value: 0 }],
  sort: { measure: "sharpe_ratio", order: "descending" },
});
const rows = csv.trim().split("\n").map((row) => row.split(","));
const table = (labels) => ({
  dates: rows.slice(1).map((row) => row[0]),
  labels,
  columns: labels.map((label) => {
    const k = rows[0].indexOf(label);
    return rows.slice(1).map((row) => Number(row[k]));
  }),
});
const assets = new Assets(table(screen.results.matches), {
  risk_free_rate: 0.04,
});
const target = assets.optimise({ method: "equal_weight" });
const result = assets.backtest({
  weights: target.results.weights,
  benchmark: table(["BENCH"]),
  rebalance: "monthly",
  trading_cost_bps: 10,
  name: "Screened funds, rebalanced monthly",
});

console.log(`${result}`);
result.show();
