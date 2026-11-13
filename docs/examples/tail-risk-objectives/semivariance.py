import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(
    method="minimum_semivariance",
    max_weight=0.5,
)

print(result)
result.show()
