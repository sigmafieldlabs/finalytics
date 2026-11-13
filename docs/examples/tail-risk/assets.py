import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
for column, label in zip(assets.returns.T, assets.labels):
    result = fl.risk.tail_risk(
        column, confidence=0.95, method="cornish_fisher"
    )
    print(
        f"{label}: VaR={result.value_at_risk:.6f} "
        f"ES={result.expected_shortfall:.6f} "
        f"clamped={str(result.shortfall_at_var).lower()}"
    )
