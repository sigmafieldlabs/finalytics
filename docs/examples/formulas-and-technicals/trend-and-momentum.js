import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  rules: [
    { measure: "price_to_sma_200", op: ">", value: 1 },
    { measure: "sma_50_above_200", op: ">=", value: 1 },
    { measure: "rsi_14", op: "between", value: [40, 70] },
    { measure: "macd_histogram", op: ">", value: 0 },
  ],
  sort: { measure: "return_6m", order: "descending" },
  limit: 10,
  name: "Positive trend without extreme RSI",
});

console.log(`${result}`);
result.show();
