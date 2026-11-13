import finalytics as fl

performance = fl.Performance.from_values(
    ["2025-12-31", "2026-01-30", "2026-02-27", "2026-03-31"],
    [1000.0, 1050.0, 1600.0, 1580.0],
    [0.0, 0.0, 500.0, 0.0],
)
for date, value in zip(performance.dates[1:], performance.returns):
    print(date, f"{value:.6f}")
