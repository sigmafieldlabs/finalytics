import { Portfolio } from "finalytics";

try {
  Portfolio.fromCsv({
    transactions:
      "date,type,amount,currency\nnot-a-date,deposit,1000,USD\n",
  });
} catch (error) {
  for (const issue of error.issues) {
    console.log(issue.severity, issue.path);
    console.log(issue.message);
  }
}
