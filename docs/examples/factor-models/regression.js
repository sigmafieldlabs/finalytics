import { regressionModel } from "finalytics";

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
const factors = { labels: ["market"], columns: [[
    0.012, -0.008, 0.015, -0.004, 0.009,
    -0.011, 0.006, 0.002, -0.006, 0.010,
]] };
const estimate = regressionModel(returns, factors);
console.log(`Method: ${estimate.method}`);
console.log(`Observations: ${estimate.observations}`);
estimate.fit.forEach((fit, index) => {
  const beta = estimate.model.exposures[index][0];
  console.log(
    `${fit.asset}: beta=${beta.toFixed(6)} ` +
    `alpha=${fit.intercept.toFixed(6)} R2=${fit.r_squared.toFixed(6)}`,
  );
});
