import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.plan(
    fl.Payments(amount=4_000, start="2046-11", end="2066-10"),
    name="Retirement income",
    start_value=250_000,
    flows=[fl.Saving(amount=1_500, every="month")],
    weights=("Balanced 60/40", {"US Equity": 0.6, "Bonds": 0.4}),
    alternatives={
        "Cautious 30/70": {"US Equity": 0.3, "Bonds": 0.7},
        "Growth 80/20": {"US Equity": 0.8, "Bonds": 0.2},
    },
    inflation=0.025,
    chance=(0.85, 0.9),
    simulations=500,
)

print(result)
result.show()
