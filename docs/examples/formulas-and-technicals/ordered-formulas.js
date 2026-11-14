import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  formulas: {
    earnings_yield: "1 / trailing_pe",
    score: "return_1y / volatility",
  },
  rules: [
    { field: "earnings_yield", op: ">", value: 0.04 },
    { field: "score", op: ">", value: 0 },
  ],
  sort: { field: "score", order: "descending" },
  limit: 10,
});

console.log(`${result}`);
result.show();
