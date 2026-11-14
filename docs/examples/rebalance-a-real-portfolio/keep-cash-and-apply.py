import finalytics as fl

portfolio = fl.datasets.portfolio()
target = portfolio.assets().optimise(
    method="minimum_variance", bounds=(0.05, 0.4)
)
result = portfolio.rebalance(target, keep_cash=True, whole_units=True)
print(result)
result.show()

after = portfolio.with_rebalance(result).report()
print(after)
after.show()
