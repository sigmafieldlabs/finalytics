import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    rules=[
        ("trailing_pe", "between", [5, 25]),
        ("price_to_book", "<", 3),
        ("market_cap", ">", 1_000_000_000),
    ],
    sort=("market_cap", "descending"),
    limit=10,
    name="Large companies at moderate valuations",
)

print(result)
result.show()
