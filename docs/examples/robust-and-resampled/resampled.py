import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(
    method="maximum_sharpe", max_weight=0.6, resample=50
)

print(result)
result.show()
