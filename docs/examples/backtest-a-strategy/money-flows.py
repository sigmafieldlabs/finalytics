import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.backtest(
    weights={"US Equity": 0.6, "Bonds": 0.4},
    start_value=10_000,
    flows={"2020-01-31": 1_000, "2021-06-15": -500},
    rebalance="yearly",
)

print(result)
result.show()
