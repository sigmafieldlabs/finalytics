import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
for point in assets.frontier(points=5, max_weight=0.50):
    print(f"{point.expected_return:.6f}", f"{point.volatility:.6f}")
