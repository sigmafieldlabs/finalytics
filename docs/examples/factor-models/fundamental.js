import { fundamentalModel } from "finalytics";

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
const estimate = fundamentalModel(
  returns,
  ["cyclical", "defensive"],
  [
    [1.0, 0.0], [1.0, 0.0],
    [0.0, 1.0], [0.0, 1.0],
],
);
console.log(`Method: ${estimate.method}`);
estimate.model.factor_covariance.labels.forEach((label, index) => {
  const column = estimate.factor_returns[index];
  console.log(
    `${label}: first=${column[0].toFixed(6)} ` +
    `last=${column.at(-1).toFixed(6)}`,
  );
});
for (const fit of estimate.fit) {
  console.log(`${fit.asset}: R2=${fit.r_squared.toFixed(6)}`);
}
