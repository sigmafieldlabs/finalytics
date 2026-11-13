import finalytics as fl

columns = [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
]
returns = list(map(list, zip(*columns)))
for method in ("ledoit_wolf", "oas", "constant_correlation"):
    estimate = fl.risk.covariance(
        returns, ["equity", "credit", "bonds"], method=method
    )
    print(
        f"{method}: shrinkage={estimate.shrinkage:.6f} "
        f"equity_variance={estimate.matrix.values[0]:.8f}"
    )
