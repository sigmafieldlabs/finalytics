import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.backtest(
    weights={"US Equity": 0.7, "Gold": 0.3},
    name="Buy and hold",
    rebalance="never",
)

print(result)
result.show()
