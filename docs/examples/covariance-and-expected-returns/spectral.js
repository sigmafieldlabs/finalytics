import { Assets, covariance, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const returns = { labels: assets.labels, columns: assets.returns };
for (const type of ["denoised", "nonlinear_shrinkage"]) {
  const estimate = covariance(returns, { type });
  console.log(
    `${type}: observations=${estimate.observations} ` +
    `first_variance=${estimate.matrix.values[0].toFixed(10)}`,
  );
}
