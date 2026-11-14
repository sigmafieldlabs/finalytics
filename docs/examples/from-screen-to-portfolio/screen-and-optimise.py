from io import StringIO

import pandas as pd
import finalytics as fl

csv = fl.datasets.file("portfolio/prices.csv")
universe = fl.Universe(
    csv,
    fields=fl.datasets.file("portfolio/instruments.csv"),
    benchmark="BENCH",
    risk_free_rate=0.04,
)
screen = universe.screen(
    rules=[("return_1y", ">", 0)],
    sort=("sharpe_ratio", "descending"),
)
print(screen)
screen.show()

prices = pd.read_csv(StringIO(csv), index_col="date", parse_dates=True)
assets = fl.Assets(prices[screen.results.matches], risk_free_rate=0.04)
result = assets.optimise(method="equal_weight", name="Screened funds")
print(result)
result.show()
