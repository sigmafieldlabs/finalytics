import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions:
    "date,type,instrument,quantity,price,amount,currency\n" +
    "2026-06-30,deposit,,,,1000,USD\n" +
    "2026-06-30,buy,VOO,1,500,,\n",
  prices: "date,VOO\n2026-06-30,500\n",
});
const found = portfolio.market.price("VOO", "2026-07-03");
console.log("Value:", found.value);
console.log("Observed:", found.date);
console.log("Carried:", found.carried);
console.log("Stale:", portfolio.market.price("VOO", "2026-07-08") == null);
