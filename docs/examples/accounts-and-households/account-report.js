import { datasets } from "finalytics";

const result = datasets.portfolio().report({ accounts: ["brokerage"] });
console.log(`${result}`.replace(/\n$/, ""));
result.show();
