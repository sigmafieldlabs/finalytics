import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
result = assets.optimise(
    method="maximum_sharpe",
    expected_returns={"US Equity": 0.04},
    max_weight=0.40,
)

print(result)
result.show()
