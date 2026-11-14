import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    measures=[
        "return_1y",
        "volatility",
        "max_drawdown",
        "sharpe_ratio",
        "relative_return_1y",
    ],
    rules=[("max_drawdown", ">", -0.25)],
    sort=("sharpe_ratio", "descending"),
    limit=5,
)

print(result)
result.show()
