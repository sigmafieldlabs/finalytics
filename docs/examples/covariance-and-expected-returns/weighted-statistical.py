import finalytics as fl

columns = [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
]
returns = list(map(list, zip(*columns)))
labels = ["equity", "credit", "bonds"]
weighted = fl.risk.covariance(returns, labels, method="ewma", decay=0.90)
statistical = fl.risk.covariance(
    returns, labels, method=fl.risk.Statistical(factors=2)
)
print(f"EWMA equity variance: {weighted.matrix.values[0]:.8f}")
print(f"EWMA credit variance: {weighted.matrix.values[4]:.8f}")
print(f"Statistical equity variance: {statistical.matrix.values[0]:.8f}")
print(f"Statistical bond variance: {statistical.matrix.values[8]:.8f}")
