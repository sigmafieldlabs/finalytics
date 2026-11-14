from io import StringIO

import pandas as pd
import finalytics as fl

csv = fl.datasets.file("portfolio/prices.csv")
universe = fl.Universe(csv, benchmark="BENCH", risk_free_rate=0.04)
screen = universe.screen(
    rules=[("return_1y", ">", 0)],
    sort=("sharpe_ratio", "descending"),
)
prices = pd.read_csv(StringIO(csv), index_col="date", parse_dates=True)
assets = fl.Assets(prices[screen.results.matches], risk_free_rate=0.04)
target = assets.optimise(method="equal_weight")
result = assets.backtest(
    weights=target.results.weights,
    benchmark=prices[["BENCH"]],
    rebalance="monthly",
    trading_cost_bps=10,
    name="Screened funds, rebalanced monthly",
)

print(result)
result.show()
