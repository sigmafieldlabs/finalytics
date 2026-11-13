import { datasets } from "finalytics";

const result = datasets.portfolio().report({
  benchmark: { VTI: 0.60, BND: 0.40 },
});
console.log(`${result}`.replace(/\n$/, ""));
result.show();
