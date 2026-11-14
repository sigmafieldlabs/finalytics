import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  as_of: "2025-12-31",
  measures: ["last_price", "return_1y", "beta"],
  rules: [{ measure: "return_1y", op: ">", value: 0 }],
  sort: { measure: "beta", order: "ascending" },
  include_benchmark: true,
  limit: 10,
});

console.log(`${result}`);
result.show();
