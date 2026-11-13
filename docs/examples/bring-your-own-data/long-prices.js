import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions:
    "date,type,instrument,quantity,price,amount,currency\n" +
    "2026-01-02,deposit,,,,1000,USD\n" +
    "2026-01-02,buy,VOO,1,500,,\n",
  prices:
    "date,instrument,price\n" +
    "2026-01-02,VOO,500\n" +
    "2026-01-05,VOO,505\n",
});
console.log(portfolio.valuation().total_base);
