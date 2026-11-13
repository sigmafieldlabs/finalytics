import { Portfolio } from "finalytics";

try {
  Portfolio.fromCsv({
    instruments: "id,kind,currency\nAAPL,equity,usd\n",
    transactions:
      "id,account,trade_date,type,instrument,quantity,price\n" +
      "t1,main,2026-01-02,buy,AAPL,ten,150\n",
    base_currency: "USD",
  });
} catch (error) {
  for (const issue of error.issues) {
    console.log(issue.path, "|", issue.message);
  }
}
