import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    rules=[
        ("sector", "in", ["Utilities"]),
        ("dividend_yield", ">", 0.03),
    ],
    sort=("sharpe_ratio", "descending"),
    limit=10,
    name="Utility income",
)

print(result)
result.show()
