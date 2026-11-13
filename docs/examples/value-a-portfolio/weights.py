import finalytics as fl

weights = fl.datasets.portfolio().weights()
for name, weight in sorted(weights.items()):
    print(name, f"{weight:.6f}")
print("Total:", f"{sum(weights.values()):.6f}")
