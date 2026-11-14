import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    formulas={
        "earnings_yield": "1 / trailing_pe",
        "score": "return_1y / volatility",
    },
    rules=[("earnings_yield", ">", 0.04), ("score", ">", 0)],
    sort=("score", "descending"),
    limit=10,
)

print(result)
result.show()
