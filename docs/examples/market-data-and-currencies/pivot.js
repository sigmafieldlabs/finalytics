import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
  fx: "date,EUR/USD,GBP/USD\n2026-06-30,1.10,1.25\n",
});
const rate = portfolio.market.fxRate("EUR", "GBP", "2026-06-30");
console.log("Pivot:", portfolio.market.pivot);
console.log("EUR/GBP:", rate.value);
