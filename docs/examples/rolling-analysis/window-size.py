import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
for window in (4, 6, 8):
    windows = fl.risk.rolling_statistics(
        returns, window=window, periods_per_year=12
    )
    print(
        f"Window {window}: count={len(windows)} "
        f"latest_volatility={windows[-1].volatility:.6f}"
    )
