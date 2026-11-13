import { statistics } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const stats = statistics(returns, {
  periods_per_year: 12,
  risk_free_rate: 0.03,
  target_return: 0.02,
});
console.log(`Observations: ${stats.observations}`);
console.log(`Total return: ${stats.total_return.toFixed(6)}`);
console.log(`Annualised return: ${stats.annualised_return.toFixed(6)}`);
console.log(`Volatility: ${stats.volatility.toFixed(6)}`);
console.log(`Downside deviation: ${stats.downside_deviation.toFixed(6)}`);
console.log(`Sharpe ratio: ${stats.sharpe_ratio.toFixed(6)}`);
console.log(`Sortino ratio: ${stats.sortino_ratio.toFixed(6)}`);
console.log(`Maximum drawdown: ${stats.max_drawdown.toFixed(6)}`);
console.log(`Ulcer index: ${stats.ulcer_index.toFixed(6)}`);
console.log(`Skewness: ${stats.skewness.toFixed(6)}`);
console.log(`Excess kurtosis: ${stats.excess_kurtosis.toFixed(6)}`);
