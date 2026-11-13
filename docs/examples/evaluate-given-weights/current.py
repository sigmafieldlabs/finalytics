import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.assets().evaluate(
    portfolio.weights(), name="Current household"
)

print(result)
result.show()
