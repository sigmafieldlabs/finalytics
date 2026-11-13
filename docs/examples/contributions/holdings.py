import finalytics as fl

performance = fl.datasets.portfolio().performance()
total = 0.0
for item in performance.contributions():
    print(item.holding, f"{item.contribution:.6f}")
    total += item.contribution
print("Holdings total:", f"{total:.6f}")
print("Portfolio return:", f"{performance.time_weighted():.6f}")
