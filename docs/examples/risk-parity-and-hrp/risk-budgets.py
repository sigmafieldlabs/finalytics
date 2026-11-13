import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
budgets = {
    "US Equity": 0.30,
    "Intl Equity": 0.20,
    "Bonds": 0.20,
    "Gold": 0.15,
    "Real Estate": 0.15,
}
result = assets.optimise(method="risk_budgeting", budgets=budgets)

print(result)
result.show()
