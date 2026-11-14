import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.plan(
    fl.Target(amount=150_000, by="2030-06"),
    name="House deposit",
    start_value=60_000,
    flows=[
        fl.Saving(amount=2_500, every="month"),
        fl.Spending(name="Car", amount=25_000, on="2028-03"),
    ],
    weights={"US Equity": 0.6, "Bonds": 0.4},
    inflation=0.025,
    fee=0.0025,
    simulations=1_000,
    currency="USD",
)

print(result)
result.show()
