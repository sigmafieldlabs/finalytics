import finalytics as fl

for timing in ("start_of_day", "end_of_day"):
    performance = fl.Performance.from_values(
        ["2026-01-30", "2026-02-27"],
        [1050.0, 1600.0], [0.0, 500.0], timing=timing,
    )
    print(timing, f"{performance.time_weighted():.6f}")
