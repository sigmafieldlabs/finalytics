import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.historical_scenario(
    "2025-02-19", "2025-04-08", proxies={"EXSA": "VTI"}
)
for holding in result.holdings:
    if holding.holding in ("VTI", "EXSA"):
        print(
            f"{holding.holding}: local={holding.return_local:.6f}, "
            f"currency={holding.currency_return:.6f}, "
            f"base={holding.return_base:.6f}"
        )
