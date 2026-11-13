import finalytics as fl

performance = fl.datasets.portfolio().performance()
contributions = performance.contributions()
total = sum(c.contribution for c in contributions) + performance.expenses()
print("Expenses:", f"{performance.expenses():.6f}")
print("With expenses:", f"{total:.6f}")
print("Return:", f"{performance.time_weighted():.6f}")
