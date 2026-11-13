import { Portfolio } from "finalytics";

const base = "id,account,trade_date,type,instrument,quantity,price,fees," +
  "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit" +
  ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n" +
  "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n";
for (const method of ["fifo", "lifo", "highest_cost", "lowest_cost"]) {
  const sale = `t4,main,2026-06-01,sell,AAPL,5,190,0,,,${method},`;
  const lines = base.trim().split("\n");
  const transactions = [lines[0], sale, ...lines.slice(1)].join("\n") + "\n";
  const portfolio = Portfolio.fromCsv({ transactions });
  console.log(method, portfolio.realisedGains().map((g) => g.lot).join(","));
}
