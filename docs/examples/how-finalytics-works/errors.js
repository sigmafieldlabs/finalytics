import { Assets, datasets } from "finalytics";

try {
  new Assets(datasets.assetClassPrices()).optimise({
    method: "target_return", max_weight: 1.5,
  });
} catch (error) {
  for (const issue of error.issues) {
    console.log(issue.path, "|", issue.message);
  }
}
