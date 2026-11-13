import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
result = assets.optimise(
    method="maximum_sharpe",
    bounds=(0.0, 0.4),
    asset_bounds={"Gold": (0.0, 0.1)},
    groups={"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)},
)

print(result)
result.show()
