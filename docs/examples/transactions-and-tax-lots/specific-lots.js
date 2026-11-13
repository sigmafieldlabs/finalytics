import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions:
    "id,account,trade_date,type,instrument,quantity,price,fees," +
    "amount,currency,lot_method,lots\n" +
    "t1,main,2026-01-02,deposit,,,,,10000,USD,,\n" +
    "t2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n" +
    "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n" +
    "t4,main,2026-06-01,sell,AAPL,5,190,0,,,specific_id," +
    "t2:2;t3:3\n",
});
for (const gain of portfolio.realisedGains()) {
  console.log(gain.lot, gain.quantity, gain.gain_local);
}
