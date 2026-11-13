import finalytics as fl

returns = [0.020, -0.010, 0.015, 0.005, -0.004, 0.012]
benchmark = [0.015, -0.008, 0.010, 0.006, -0.003, 0.009]
found = fl.risk.relative_statistics(returns, benchmark, periods_per_year=12)
print("Upside capture:", f"{found.upside_capture:.6f}")
print("Downside capture:", f"{found.downside_capture:.6f}")
print("Beta:", f"{found.beta:.6f}")
