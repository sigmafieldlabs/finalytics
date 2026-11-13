import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
result = assets.optimise(method="minimum_variance", cash=(0, 0.20))

print(result)
