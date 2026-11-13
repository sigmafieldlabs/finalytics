import { Portfolio } from "finalytics";

const portfolio = Portfolio.fromCsv({
  instruments: "id,kind,currency\nSAP,equity,EUR\n",
  transactions:
    "date,type,instrument,quantity,price,amount,currency\n" +
    "2026-06-30,deposit,,,,2000,USD\n" +
    "2026-06-30,transfer_in,SAP,10,,,\n",
  prices: "date,SAP\n2026-06-30,200\n",
  fx: "date,EUR/USD\n2026-06-30,1.10\n",
});
const position = portfolio.valuation().positions[0];
console.log("Local:", position.market_value_local);
console.log("Base:", position.market_value_base);
