import { datasets, reportTables } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.report({ benchmark: "BENCH" });
console.log(`${result}`);
result.show(undefined, "dark");

for (const table of reportTables(result)) {
  console.log(`Table: ${table.name} (${table.rows.length} rows)`);
}
