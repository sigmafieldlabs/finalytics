import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.compare(
    methods=["minimum_variance", "maximum_diversification"],
    validation=fl.WalkForward(years=10, expanding=True),
    rebalance="yearly",
    trading_cost_bps=10,
)

print(result)
result.show()
