import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
portfolio = fl.datasets.portfolio()
universe = fl.datasets.universe()

print("Assets:", len(assets.labels), assets.labels[:2])
print("Portfolio:", portfolio.base_currency, len(portfolio.data.transactions))
print("Universe:", len(universe), universe.benchmark)
