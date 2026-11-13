import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.evaluate(
    {"US Equity": 0.6, "Bonds": 0.4},
    max_weight=0.5,
    frontier=True,
    name="Sixty forty",
)

print(result)
result.show()
