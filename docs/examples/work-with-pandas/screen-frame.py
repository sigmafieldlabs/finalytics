import finalytics as fl
import finalytics.pandas as flpd

universe = fl.datasets.universe()
screen = universe.screen(
    rules=[("return_1y", ">", 0)],
    sort=("sharpe_ratio", "descending"),
    limit=10,
)
frame = flpd.securities(screen)
columns = ["matched", "rank", "return_1y", "sharpe_ratio", "sector"]
print(frame.loc[frame.matched, columns].round(6).to_string())
