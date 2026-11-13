import finalytics as fl

returns = [0.020, -0.010, 0.015, 0.005, -0.004, 0.012]
benchmark = [0.015, -0.008, 0.010, 0.006, -0.003, 0.009]
found = fl.risk.relative_statistics(returns, benchmark, periods_per_year=12)
print("Active return:", f"{found.active_return:.6f}")
print("Tracking error:", f"{found.tracking_error:.6f}")
print("Information ratio:", f"{found.information_ratio:.6f}")
