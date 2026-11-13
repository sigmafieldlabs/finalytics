import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
weights = {
    "US Equity": 1.2,
    "Intl Equity": -0.3,
    "Bonds": 0.3,
    "Gold": -0.1,
    "Real Estate": -0.1,
}
result = assets.evaluate(weights, name="Long short evaluation")

print(result)
result.show()
