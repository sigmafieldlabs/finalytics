import { statisticalModel } from "finalytics";

const returns = {
  labels: ["tech", "industrials", "utilities", "bonds"],
  columns: [
    [0.0176, -0.0114, 0.0205, -0.0052, 0.0097,
     -0.0133, 0.0078, 0.0016, -0.0058, 0.0120],
    [0.0110, -0.0060, 0.0150, -0.0060, 0.0100,
     -0.0110, 0.0070, 0.0010, -0.0060, 0.0110],
    [0.0084, -0.0076, 0.0125, -0.0018, 0.0063,
     -0.0087, 0.0062, 0.0014, -0.0052, 0.0080],
    [0.0034, -0.0016, 0.0020, 0.0012, 0.0008,
     -0.0012, -0.0008, 0.0014, -0.0012, 0.0010],
],
};
const estimate = statisticalModel(returns, 2);
console.log(`Method: ${estimate.method}`);
estimate.explained_variance.forEach((share, index) => {
  console.log(`pc${index + 1}: explained=${share.toFixed(6)}`);
});
estimate.model.assets.forEach((label, index) => {
  const exposures = estimate.model.exposures[index];
  console.log(
    `${label}: pc1=${exposures[0].toFixed(6)} ` +
    `pc2=${exposures[1].toFixed(6)}`,
  );
});
