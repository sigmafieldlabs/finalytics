import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.backtest(
    method="maximum_sharpe",
    max_weight=0.4,
    benchmark={"US Equity": 0.6, "Bonds": 0.4},
    validation=fl.Holdout(start="2016-11-01"),
    rebalance="quarterly",
    trading_cost_bps=10,
)

print(result)
result.show()
