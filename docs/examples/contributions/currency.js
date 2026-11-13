import { datasets } from "finalytics";

for (const item of datasets.portfolio().performance().contributions()) {
  if (item.holding === "EXSA") {
    console.log("Local:", item.return_local.toFixed(6));
    console.log("Base:", item.return_base.toFixed(6));
    console.log("Currency effect:", item.currency_effect.toFixed(6));
  }
}
