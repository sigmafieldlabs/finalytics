import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
path = fl.GlidePath(
    points={
        "2026-11": {"US Equity": 0.8, "Bonds": 0.2},
        "2030-01": {"US Equity": 0.3, "Bonds": 0.7},
    }
)
result = assets.plan(
    fl.Target(amount=150_000, by="2030-06"),
    start_value=60_000,
    flows=[fl.Saving(amount=2_500, every="month")],
    weights=path,
    inflation=0.025,
    model="normal",
    simulations=500,
    seed=7,
)

print(result)
result.show()
