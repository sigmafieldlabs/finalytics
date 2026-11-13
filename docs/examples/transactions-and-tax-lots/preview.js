import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: "id,account,trade_date,type,instrument,quantity,price,fees," +
  "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit" +
  ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n" +
  "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n",
});
for (const method of ["fifo", "highest_cost"]) {
  const lots = portfolio.lotsForSale("2026-06-30", "main", "AAPL", 5, method);
  console.log(method, lots.map((lot) =>
    `${lot.lot}:${lot.quantity}`).join(","));
}
console.log("Open lots:", portfolio.holdings().lots.length);
