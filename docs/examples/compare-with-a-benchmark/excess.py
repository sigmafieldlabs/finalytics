import finalytics as fl

performance = fl.Performance.from_values(
    ["2025-12-31", "2026-03-31"], [1000.0, 1070.323]
)
excess = performance.excess_return(
    [100.0, 104.0], "ITD", dates=["2025-12-31", "2026-03-31"]
)
print("Portfolio:", f"{excess.portfolio:.6f}")
print("Benchmark:", f"{excess.benchmark:.6f}")
print("Arithmetic:", f"{excess.arithmetic:.6f}")
print("Geometric:", f"{excess.geometric:.6f}")
