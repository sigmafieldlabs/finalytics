import { Portfolio, datasets } from "finalytics";

const original = datasets.portfolio();
const restored = Portfolio.fromJson(original.toJson());
console.log("Equal values:", original.valuation().total_base ===
  restored.valuation().total_base);
console.log("Transactions:", restored.data.transactions.length);
