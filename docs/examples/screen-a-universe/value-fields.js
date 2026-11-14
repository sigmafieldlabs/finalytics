import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  rules: [
    { field: "trailing_pe", op: "between", value: [5, 25] },
    { field: "price_to_book", op: "<", value: 3 },
    { field: "market_cap", op: ">", value: 1000000000 },
  ],
  sort: { field: "market_cap", order: "descending" },
  limit: 10,
  name: "Large companies at moderate valuations",
});

console.log(`${result}`);
result.show();
