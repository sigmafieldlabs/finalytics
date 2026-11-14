import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    rules=[
        ("price_to_sma_200", ">", 1),
        ("sma_50_above_200", ">=", 1),
        ("rsi_14", "between", [40, 70]),
        ("macd_histogram", ">", 0),
    ],
    sort=("return_6m", "descending"),
    limit=10,
    name="Positive trend without extreme RSI",
)

print(result)
result.show()
