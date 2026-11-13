import finalytics as fl

performance = fl.datasets.portfolio().performance()
levels = fl.datasets.report_options().benchmark
print("Benchmark column:", levels)
excess = performance.excess_return(
    fl.datasets.portfolio().market.prices["BENCH"].values,
    "ITD", dates=fl.datasets.portfolio().market.price_dates,
)
print("Arithmetic:", f"{excess.arithmetic:.6f}")
print("Geometric:", f"{excess.geometric:.6f}")
