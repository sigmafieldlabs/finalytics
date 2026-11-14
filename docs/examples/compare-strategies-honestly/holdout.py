import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.compare(
    methods=["minimum_variance", "maximum_sharpe"],
    weights={"60/40": {"US Equity": 0.6, "Bonds": 0.4}},
    validation=fl.Holdout(start="2016-11-01"),
    rebalance="quarterly",
    trading_cost_bps=10,
    max_weight=0.6,
)

print(result)
result.show()
