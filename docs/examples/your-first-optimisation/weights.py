import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.optimise(method="minimum_variance")
for name in sorted(result.results.weights):
    print(name, f"{result.results.weights[name]:.6f}")
print("Status:", result.results.status)
