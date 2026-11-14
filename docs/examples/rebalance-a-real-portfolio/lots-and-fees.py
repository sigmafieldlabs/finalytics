import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.rebalance(
    {"VTI": 0.35, "EXSA": 0.1, "BND": 0.35, "GLD": 0.1, "VNQ": 0.1},
    accounts=["brokerage"],
    lot_method="highest_cost",
    whole_units=True,
    fee_per_trade=2,
    fee_bps=5,
    minimum_fee=1,
    minimum_trade=100,
)

print(result)
result.show()
