import finalytics as fl

segments = [
    ("UK", 0.4, 0.20, 0.4, 0.10),
    ("Japan", 0.3, -0.05, 0.2, -0.04),
    ("US", 0.3, 0.06, 0.4, 0.08),
]
result = fl.performance.brinson_fachler([
    fl.performance.attribution_period(
        "2026-01-01", "2026-12-31", segments
    )
])
print("Excess:", f"{result.excess_return:.6f}")
for segment in result.segments:
    print(segment.name, f"{segment.allocation:.6f}",
          f"{segment.selection:.6f}", f"{segment.interaction:.6f}")
