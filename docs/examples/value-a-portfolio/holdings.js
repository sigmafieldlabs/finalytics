import { datasets } from "finalytics";

const holdings = datasets.portfolio().holdings("2026-10-30");
for (const position of holdings.positions) {
  console.log(position.account, position.instrument, position.quantity,
    position.cost_base);
}
