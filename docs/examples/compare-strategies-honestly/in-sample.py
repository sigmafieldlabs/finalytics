import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.compare(
    methods=["equal_weight", "minimum_variance", "maximum_sharpe"],
    max_weight=0.4,
    name="In-sample illustration",
)

print(result)
result.show()
