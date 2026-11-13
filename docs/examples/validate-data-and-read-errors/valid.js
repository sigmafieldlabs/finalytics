import { Portfolio, validatePortfolio } from "finalytics";

const data = Portfolio.fromCsv({
  transactions: "date,type,amount,currency\n2026-01-02,deposit,1000,USD\n",
});
console.log("Issues:", JSON.stringify(validatePortfolio(data.data)));
