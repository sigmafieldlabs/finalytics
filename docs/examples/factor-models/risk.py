import finalytics as fl

model = fl.risk.factor_model(
    [[1.0, 0.0], [1.2, 0.0], [0.0, -6.0]],
    [[0.04, -0.0016], [-0.0016, 0.0001]],
    [0.01, 0.02, 0.0001],
    assets=["SPY", "SAP", "BOND"],
    factors=["equity", "rates"],
)
weights = [0.4, 0.3, 0.3]
benchmark = [0.3, 0.3, 0.4]
for name, result in (
    ("Portfolio", fl.risk.factor_risk(weights, model)),
    (
        "Active",
        fl.risk.factor_tracking_error(weights, benchmark, model),
    ),
):
    print(f"{name}: volatility={result.volatility:.6f}")
    for item in result.factors:
        print(
            f"  {item.factor}: exposure={item.exposure:.6f} "
            f"component={item.component:.6f} share={item.share:.6f}"
        )
    print(f"  specific: share={result.specific_share:.6f}")
