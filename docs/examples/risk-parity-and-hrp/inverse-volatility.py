import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(method="inverse_volatility")

print(result)
result.show()
