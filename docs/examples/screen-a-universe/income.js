import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  rules: [
    { field: "sector", op: "in", values: ["Utilities"] },
    { measure: "dividend_yield", op: ">", value: 0.03 },
  ],
  sort: { measure: "sharpe_ratio", order: "descending" },
  limit: 10,
  name: "Utility income",
});

console.log(`${result}`);
result.show();
