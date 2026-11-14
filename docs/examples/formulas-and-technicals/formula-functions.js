import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  formulas: {
    log_size: "log(market_cap)",
    value_score: "sqrt(max(trailing_pe, 0)) / price_to_book",
    risk_score: "abs(return_1y) / max(volatility, 0.01)",
  },
  rules: [
    { field: "value_score", op: "<", value: 5 },
    { field: "risk_score", op: "<", value: 2 },
  ],
  sort: { field: "log_size", order: "descending" },
  limit: 10,
});

console.log(`${result}`);
result.show();
