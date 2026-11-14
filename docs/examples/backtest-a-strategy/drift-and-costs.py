import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.backtest(
    weights={"US Equity": 0.6, "Bonds": 0.4},
    name="Balanced",
    rebalance=fl.Drift(band=0.05),
    trading_cost_bps=25,
)

print(result)
result.show()
