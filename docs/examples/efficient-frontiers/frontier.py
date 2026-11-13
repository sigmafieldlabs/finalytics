import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
frontier = assets.frontier(points=6, max_weight=0.5)
for point in frontier:
    print(
        f"return={point.expected_return:.6f}, "
        f"volatility={point.volatility:.6f}"
    )
