import finalytics as fl

methods = [
    "historical",
    "james_stein",
    fl.risk.EwmaMean(decay=0.97),
]
for method in methods:
    assets = fl.Assets(
        fl.datasets.asset_class_prices(),
        risk_free_asset="T-Bills",
        mean_returns=method,
    )
    name = method if isinstance(method, str) else "ewma"
    print(name)
    for label, value in assets.mean_returns().items():
        print(f"  {label}: {value:.6f}")
