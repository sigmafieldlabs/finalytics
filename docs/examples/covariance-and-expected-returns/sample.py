import finalytics as fl

columns = [
    [0.012, -0.008, 0.015, -0.004, 0.009, -0.011],
    [0.004, -0.002, 0.006, 0.001, 0.003, -0.005],
    [-0.001, 0.003, 0.000, 0.002, -0.002, 0.004],
]
returns = list(map(list, zip(*columns)))
estimate = fl.risk.covariance(
    returns, ["equity", "credit", "bonds"], method="sample"
)
print(f"Observations: {estimate.observations}")
for label, row in zip(
    estimate.matrix.labels,
    fl.risk.to_array(estimate.matrix),
):
    print(label, " ".join(f"{value:.8f}" for value in row))
annual = fl.risk.scaled(estimate.matrix, 252)
print(f"Annual equity variance: {annual.values[0]:.8f}")
