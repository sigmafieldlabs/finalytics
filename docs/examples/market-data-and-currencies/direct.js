import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: "date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
  fx: "date,EUR/USD\n2026-06-30,1.10\n",
});
const rate = portfolio.market.fxRate("EUR", "USD", "2026-06-30");
console.log("EUR/USD:", Number(rate.value).toFixed(2));
console.log("100 EUR in USD:", (100 * Number(rate.value)).toFixed(2));
