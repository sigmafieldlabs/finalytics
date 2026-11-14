import finalytics as fl
import finalytics.pandas as flpd

performance = fl.datasets.portfolio().performance()
for series in [
    flpd.returns(performance),
    flpd.growth(performance),
    flpd.values(performance),
]:
    print(series.head().round(6).to_string())
    print()
