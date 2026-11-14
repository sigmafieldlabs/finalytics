import { Assets, Universe, datasets } from "finalytics";

const csv = datasets.file("portfolio/prices.csv");
const universe = Universe.fromCsv(csv, {
  fields: datasets.file("portfolio/instruments.csv"),
  benchmark: "BENCH",
  risk_free_rate: 0.04,
});
const screen = universe.screen({
  rules: [{ measure: "return_1y", op: ">", value: 0 }],
  sort: { measure: "sharpe_ratio", order: "descending" },
});
console.log(`${screen}`);
screen.show();

const rows = csv.trim().split("\n").map((row) => row.split(","));
const labels = screen.results.matches;
const columns = labels.map((label) => {
  const k = rows[0].indexOf(label);
  return rows.slice(1).map((row) => Number(row[k]));
});
const assets = new Assets({
  dates: rows.slice(1).map((row) => row[0]),
  labels,
  columns,
}, { risk_free_rate: 0.04 });
const result = assets.optimise({
  method: "equal_weight",
  name: "Screened funds",
});
console.log(`${result}`);
result.show();
