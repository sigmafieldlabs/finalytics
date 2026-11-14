import { datasets } from "finalytics";

const universe = datasets.universe();
const result = universe.screen({
  rules: [
    { measure: "from_52w_high", op: ">", value: -0.1 },
    { measure: "from_52w_low", op: ">", value: 0.1 },
    { measure: "volume_ratio", op: ">", value: 1 },
    { measure: "dollar_volume_20d", op: ">", value: 1000000 },
  ],
  sort: { measure: "volume_ratio", order: "descending" },
  limit: 10,
});

console.log(`${result}`);
result.show();
