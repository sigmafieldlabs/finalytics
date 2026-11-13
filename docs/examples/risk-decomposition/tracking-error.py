import finalytics as fl

covariance = fl.risk.matrix(
    [
        [0.0400, 0.0100, -0.0060],
        [0.0100, 0.0900, -0.0090],
        [-0.0060, -0.0090, 0.0100],
    ],
    ["equity", "credit", "bonds"],
)
result = fl.risk.tracking_error_decomposition(
    [0.6, 0.2, 0.2], [0.4, 0.4, 0.2], covariance
)
print(f"Tracking error: {result.volatility:.6f}")
for item in result.contributions:
    print(
        f"{item.label}: active_weight={item.weight:.6f} "
        f"component={item.component:.6f} share={item.share:.6f}"
    )
