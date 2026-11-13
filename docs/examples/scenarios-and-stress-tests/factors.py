import finalytics as fl

portfolio = fl.datasets.portfolio()
model = fl.risk.factor_model(
    [
        [1.0, 0.0],
        [1.1, 0.0],
        [0.0, -6.0],
        [0.3, 0.0],
        [0.8, -1.0],
    ],
    [[0.04, -0.0016], [-0.0016, 0.0001]],
    [0.01, 0.01, 0.0025, 0.01, 0.01],
    assets=["VTI", "EXSA", "BND", "GLD", "VNQ"],
    factors=["equity", "rates"],
)
result = portfolio.shock_scenario(
    factors={"equity": -0.15},
    factor_model=model,
    propagate=True,
)
for factor, value in result.factor_returns.items():
    print(f"{factor}: {value:.6f}")
for holding in result.holdings[:5]:
    print(f"{holding.holding}: local={holding.return_local:.6f}")
