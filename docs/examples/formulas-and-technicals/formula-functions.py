import finalytics as fl

universe = fl.datasets.universe()
result = universe.screen(
    formulas={
        "log_size": "log(market_cap)",
        "value_score": "sqrt(max(trailing_pe, 0)) / price_to_book",
        "risk_score": "abs(return_1y) / max(volatility, 0.01)",
    },
    rules=[("value_score", "<", 5), ("risk_score", "<", 2)],
    sort=("log_size", "descending"),
    limit=10,
)

print(result)
result.show()
