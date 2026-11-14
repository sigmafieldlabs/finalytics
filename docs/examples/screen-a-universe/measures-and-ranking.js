import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  measures: [
    "return_1y",
    "volatility",
    "max_drawdown",
    "sharpe_ratio",
    "relative_return_1y",
  ],
  rules: [{ measure: "max_drawdown", op: ">", value: -0.25 }],
  sort: { measure: "sharpe_ratio", order: "descending" },
  limit: 5,
});

console.log(`${result}`);
result.show();
