import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
optimised = assets.optimise(
    method="maximum_sharpe", max_weight=0.4
)
result = assets.evaluate(
    optimised.results.weights, name="Optimised weights"
)

print(result)
result.show()
