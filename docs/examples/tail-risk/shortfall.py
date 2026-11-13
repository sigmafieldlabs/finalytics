import finalytics as fl

scenarios = [
    [-0.08, 0.01],
    [0.02, 0.00],
    [0.03, 0.005],
    [-0.01, -0.004],
]
result = fl.risk.shortfall_decomposition(
    [0.6, 0.4], scenarios, ["equity", "bonds"], confidence=0.75
)
print(f"Expected shortfall: {result.expected_shortfall:.6f}")
for item in result.contributions:
    print(
        f"{item.label}: component={item.component:.6f} "
        f"share={item.share:.6f}"
    )
print(f"Component sum: {sum(c.component for c in result.contributions):.6f}")
