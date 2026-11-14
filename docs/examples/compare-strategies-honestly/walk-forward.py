import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.compare(
    methods=["equal_weight", "minimum_variance", "maximum_sharpe"],
    validation=fl.WalkForward(years=5),
    rebalance="quarterly",
    trading_cost_bps=10,
    max_weight=0.4,
)

print(result)
result.show()
