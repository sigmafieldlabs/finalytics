import finalytics as fl

portfolio = fl.datasets.portfolio()
current = portfolio.weights()
result = portfolio.assets().optimise(
    method="maximum_sharpe",
    current_weights=current,
    max_turnover=0.15,
    keep_cash=True,
)

print(result)
result.show()
