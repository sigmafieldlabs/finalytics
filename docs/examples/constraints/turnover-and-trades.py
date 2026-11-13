import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
current = {
    "US Equity": 0.40,
    "Intl Equity": 0.10,
    "Bonds": 0.30,
    "Gold": 0.10,
    "Real Estate": 0.05,
    "T-Bills": 0.05,
}
result = assets.optimise(
    method="maximum_sharpe",
    current_weights=current,
    max_turnover=0.15,
    min_trade=0.02,
    max_trades=3,
)

print(result)
result.show()
