import finalytics as fl

periods = [
    ("2026-01-01", "2026-04-01",
     [("Equities", 0.6, 0.08, 0.5, 0.06),
      ("Bonds", 0.4, 0.01, 0.5, 0.015)]),
    ("2026-04-01", "2026-07-01",
     [("Equities", 0.65, -0.05, 0.5, -0.04),
      ("Bonds", 0.35, 0.02, 0.5, 0.02)]),
]
result = fl.performance.brinson_fachler([
    fl.performance.attribution_period(start, end, segments)
    for start, end, segments in periods
])
print("Excess:", f"{result.excess_return:.6f}")
effects = result.allocation + result.selection + result.interaction
print("Effects:", f"{effects:.6f}")
for period in result.periods:
    print(period.start, f"{period.linking_coefficient:.6f}")
