import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(
    method="hierarchical_risk_parity", linkage="ward"
)

print(result)
result.show()
