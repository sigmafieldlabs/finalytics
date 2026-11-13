import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
windows = fl.risk.rolling_statistics(
    returns, window=6, periods_per_year=12, risk_free_rate=0.03
)
print(f"Windows: {len(windows)}")
for index in (0, len(windows) - 1):
    stats = windows[index]
    print(
        f"Window {index}: return={stats.annualised_return:.6f} "
        f"volatility={stats.volatility:.6f} Sharpe={stats.sharpe_ratio:.6f}"
    )
