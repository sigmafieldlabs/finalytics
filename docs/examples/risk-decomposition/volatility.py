import finalytics as fl

covariance = fl.risk.matrix(
    [
        [0.0400, 0.0100, -0.0060],
        [0.0100, 0.0900, -0.0090],
        [-0.0060, -0.0090, 0.0100],
    ],
    ["equity", "credit", "bonds"],
)
result = fl.risk.risk_decomposition([0.5, 0.3, 0.2], covariance)
print(f"Volatility: {result.volatility:.6f}")
for item in result.contributions:
    print(
        f"{item.label}: marginal={item.marginal:.6f} "
        f"component={item.component:.6f} share={item.share:.6f}"
    )
print(f"Component sum: {sum(c.component for c in result.contributions):.6f}")
print(f"Share sum: {sum(c.share for c in result.contributions):.6f}")
