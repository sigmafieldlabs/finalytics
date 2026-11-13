import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
  fx: "date,USD/EUR\n2026-06-30,0.80\n",
});
console.log(portfolio.market.fxRate("EUR", "USD", "2026-06-30").value);
