import { statistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
for (const target of [0.0, 0.04]) {
  const stats = statistics(returns, {
    periods_per_year: 12,
    target_return: target,
  });
  const downside = stats.downside_deviation.toFixed(6);
  const sortino = stats.sortino_ratio.toFixed(6);
  console.log(
    `Target ${target.toFixed(2)}: downside=${downside} ` +
    `Sortino=${sortino}`,
  );
}
const constant = statistics(Array(12).fill(0.01), { periods_per_year: 12 });
console.log(`Constant Sharpe: ${constant.sharpe_ratio ?? "None"}`);
console.log(`Constant Sortino: ${constant.sortino_ratio ?? "None"}`);
