import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
for target in (0.0, 0.04):
    stats = fl.risk.statistics(
        returns, periods_per_year=12, target_return=target
    )
    print(
        f"Target {target:.2f}: downside={stats.downside_deviation:.6f} "
        f"Sortino={stats.sortino_ratio:.6f}"
    )
constant = fl.risk.statistics([0.01] * 12, periods_per_year=12)
print(f"Constant Sharpe: {constant.sharpe_ratio}")
print(f"Constant Sortino: {constant.sortino_ratio}")
