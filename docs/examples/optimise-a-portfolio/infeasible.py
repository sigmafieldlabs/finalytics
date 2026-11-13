import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
result = assets.optimise(
    method="target_return", target=0.20, max_weight=0.40
)

print(result.results.status)
