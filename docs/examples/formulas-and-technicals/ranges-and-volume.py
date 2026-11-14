import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    rules=[
        ("from_52w_high", ">", -0.1),
        ("from_52w_low", ">", 0.1),
        ("volume_ratio", ">", 1),
        ("dollar_volume_20d", ">", 1_000_000),
    ],
    sort=("volume_ratio", "descending"),
    limit=10,
)

print(result)
result.show()
