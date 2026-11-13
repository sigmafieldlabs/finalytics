import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.optimise(method="minimum_variance")

print("Expected return:", f"{result.results.expected_return:.6f}")
print("Volatility:", f"{result.results.volatility:.6f}")
print(result, end="")
result.show()
