import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    as_of="2025-12-31",
    measures=["last_price", "return_1y", "beta"],
    rules=[("return_1y", ">", 0)],
    sort=("beta", "ascending"),
    include_benchmark=True,
    limit=10,
)

print(result)
result.show()
