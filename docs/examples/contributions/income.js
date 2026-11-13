import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
for (const item of portfolio.performance().contributions()) {
  if (item.holding === "VTI") {
    console.log("Holding:", item.holding);
    console.log("Contribution:", item.contribution.toFixed(6));
    console.log("Own return:", item.return_base.toFixed(6));
  }
}
console.log("Events:", portfolio.events().length);
