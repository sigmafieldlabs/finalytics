import finalytics as fl

performance = fl.Performance.from_values(
    ["2023-03-31", "2026-03-31"], [100.0, 133.1]
)
found = performance.period("3Y")
print("Cumulative:", f"{found.cumulative:.6f}")
print("Annualised:", f"{found.annualised:.6f}")
short = fl.datasets.portfolio().performance().period("1Y")
print("One year:", f"{short.cumulative:.6f}")
