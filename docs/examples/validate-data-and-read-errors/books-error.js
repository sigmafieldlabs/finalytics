import { Portfolio } from "finalytics";

try {
  Portfolio.fromCsv({
    transactions:
      "date,type,instrument,quantity,price,amount,currency\n" +
      "2026-01-02,deposit,,,,1000,USD\n" +
      "2026-01-02,sell,VOO,2,500,,\n",
  });
} catch (error) {
  for (const issue of error.issues) {
    console.log(issue.path, "|", issue.message);
  }
}
