import finalytics as fl

portfolio = fl.datasets.portfolio()
target = portfolio.assets().optimise(
    method="minimum_variance", bounds=(0.05, 0.4)
)
result = portfolio.rebalance(
    target,
    whole_units=True,
    fee_bps=5,
    minimum_fee=1,
    tax=(0.37, 0.2),
)

print(result)
result.show()
