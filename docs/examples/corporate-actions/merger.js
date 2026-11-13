import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  transactions: "id,date,type,instrument,new_instrument,quantity,price,from" +
  ",to,ratio,cost_allocation,cash_per_unit,amount,currency\nt1" +
  ",2026-01-02,deposit,,,,,,,,,,10000,USD\nt2,2026-01-02,buy,A" +
  "AA,,10,100,,,,,,,\nt3,2026-06-01,merger,AAA,BBB,,,,,0.5,,10" +
  ",,\n",
});
for (const lot of portfolio.holdings().lots) {
  console.log(lot.id, lot.instrument, lot.quantity, lot.cost_local);
}
for (const gain of portfolio.realisedGains()) {
  console.log("Gain:", gain.lot, gain.gain_local);
}
