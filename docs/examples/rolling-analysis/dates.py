import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
dates = [
    "2026-01-30", "2026-02-27", "2026-03-31", "2026-04-30",
    "2026-05-29", "2026-06-30", "2026-07-31", "2026-08-31",
    "2026-09-30", "2026-10-30", "2026-11-30", "2026-12-31",
]
window = 6
windows = fl.risk.rolling_statistics(returns, window, periods_per_year=12)
for offset, stats in enumerate(windows[-3:]):
    end = dates[window - 1 + len(windows) - 3 + offset]
    print(f"Ending {end}: volatility={stats.volatility:.6f}")
