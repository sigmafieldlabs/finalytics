import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.shock_scenario(
    groups={("asset_class", "Equity"): -0.20},
    currencies={"EUR": -0.10},
)
for holding in result.holdings:
    if holding.return_local != 0 or holding.currency_return != 0:
        print(
            f"{holding.holding}: local={holding.return_local:.6f}, "
            f"currency={holding.currency_return:.6f}, "
            f"base={holding.return_base:.6f}"
        )
