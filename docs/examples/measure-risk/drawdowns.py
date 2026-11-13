import finalytics as fl

levels = [100.0, 95.0, 101.0, 90.0, 92.0, 104.0]
dates = [
    "2026-01-02", "2026-01-05", "2026-01-06",
    "2026-01-07", "2026-01-08", "2026-01-09",
]
for item in fl.risk.drawdowns(levels, dates):
    recovery = item.recovery.isoformat() if item.recovery else "open"
    print(
        f"{item.peak} {item.trough} {recovery} "
        f"{item.depth:.6f} {item.days_to_trough} {item.days_to_recovery}"
    )
