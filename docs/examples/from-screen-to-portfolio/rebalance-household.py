import finalytics as fl

portfolio = fl.datasets.portfolio()
universe = fl.Universe(
    fl.datasets.file("portfolio/prices.csv"),
    fields=fl.datasets.file("portfolio/instruments.csv"),
    benchmark="BENCH",
    risk_free_rate=0.04,
)
screen = universe.screen(
    rules=[("return_1y", ">", 0)],
    sort=("sharpe_ratio", "descending"),
)
weight = 1 / len(screen.results.matches)
targets = {asset: weight for asset in screen.results.matches}
result = portfolio.rebalance(
    targets,
    whole_units=True,
    fee_bps=5,
    tax=(0.37, 0.2),
    name="Rebalance to screened funds",
)

print(result)
result.show()
