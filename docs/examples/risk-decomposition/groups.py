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
groups = {
    "equity": "Risk assets",
    "credit": "Risk assets",
    "bonds": "Defensive",
}
for item in fl.risk.group_risk(result, lambda _, label: groups[label]):
    print(
        f"{item.group}: weight={item.weight:.6f} "
        f"component={item.component:.6f} share={item.share:.6f}"
    )
