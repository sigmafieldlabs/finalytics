import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
stats = fl.risk.statistics(
    returns, periods_per_year=12, risk_free_rate=0.03, target_return=0.02
)
print(f"Observations: {stats.observations}")
print(f"Total return: {stats.total_return:.6f}")
print(f"Annualised return: {stats.annualised_return:.6f}")
print(f"Volatility: {stats.volatility:.6f}")
print(f"Downside deviation: {stats.downside_deviation:.6f}")
print(f"Sharpe ratio: {stats.sharpe_ratio:.6f}")
print(f"Sortino ratio: {stats.sortino_ratio:.6f}")
print(f"Maximum drawdown: {stats.max_drawdown:.6f}")
print(f"Ulcer index: {stats.ulcer_index:.6f}")
print(f"Skewness: {stats.skewness:.6f}")
print(f"Excess kurtosis: {stats.excess_kurtosis:.6f}")
