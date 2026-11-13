import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(
    method="maximum_sharpe",
    benchmark_weights={"US Equity": 0.6, "Bonds": 0.4},
    max_tracking_error=0.08,
)

print(result)
result.show()
