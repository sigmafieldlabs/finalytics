import finalytics as fl

performance = fl.datasets.portfolio().performance()
for name in ("MTD", "QTD", "YTD", "1Y", "ITD"):
    found = performance.period(name)
    if found is None:
        print(name, "insufficient history")
    else:
        print(name, found.start, f"{found.cumulative:.6f}")
