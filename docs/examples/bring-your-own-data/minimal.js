import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions:
    "date,type,instrument,quantity,price,amount,currency\n" +
    "2026-01-02,deposit,,,,20000,USD\n" +
    "2026-01-02,buy,VOO,20,500,,\n" +
    "2026-01-02,buy,AGG,100,98,,\n",
  prices: "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n",
});
console.log("Base currency:", portfolio.baseCurrency);
console.log("Value:", portfolio.valuation().total_base);
