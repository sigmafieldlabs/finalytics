import datetime

import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.plan(
    end=datetime.date(2040, 12, 1),
    start_value=10_000,
    flows=[fl.Saving(amount=500, every="month")],
    weights={"US Equity": 1.0},
    inflation=0.02,
    model=fl.Bootstrap(block=6),
    simulations=200,
)

print(result)
result.show()
