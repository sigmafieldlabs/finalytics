import { Assets, datasets } from "finalytics";

try {
  new Assets(datasets.assetClassPrices()).optimise({
    method: "target_return",
  });
} catch (error) {
  console.log(error.name);
  console.log("Issues:", error.issues.length);
}
