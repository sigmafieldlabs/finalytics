import { readFile } from "node:fs/promises";
import { Portfolio } from "finalytics";

const transactions = await readFile("transactions.csv", "utf8");
const prices = await readFile("prices.csv", "utf8");
const fx = await readFile("fx.csv", "utf8");
const portfolio = Portfolio.fromCsv({ transactions, prices, fx });
console.log("Transactions:", portfolio.data.transactions.length);
console.log("Value:", portfolio.valuation().total_base);
