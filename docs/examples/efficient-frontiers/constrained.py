import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
frontier = assets.frontier(
    points=5,
    bounds=(0.0, 0.5),
    groups={"Equity": (["US Equity", "Intl Equity"], 0.2, 0.7)},
    cash=(0.0, 0.2),
)
for point in frontier:
    print(
        f"return={point.expected_return:.6f}, "
        f"volatility={point.volatility:.6f}"
    )
    for asset in sorted(point.weights):
        print(f"  {asset}: {point.weights[asset]:.6f}")
