import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
benchmark = [
    0.015, -0.010, 0.025, 0.010, -0.030, 0.012,
    0.020, -0.005, 0.010, 0.022, -0.018, 0.011,
]
windows = fl.risk.rolling_relative_statistics(
    returns, benchmark, window=6, periods_per_year=12
)
for index, stats in enumerate(windows):
    print(
        f"Window {index}: active={stats.active_return:.6f} "
        f"TE={stats.tracking_error:.6f} beta={stats.beta:.6f} "
        f"correlation={stats.correlation:.6f}"
    )
