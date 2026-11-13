import finalytics as fl

covariance = fl.risk.matrix(
    [
        [0.0400, 0.0100, 0.0000],
        [0.0100, 0.0900, 0.0000],
        [0.0000, 0.0000, 0.0025],
    ],
    ["equity", "credit", "bonds"],
)
correlation = fl.risk.correlation(covariance)
for label, row in zip(correlation.labels, fl.risk.to_array(correlation)):
    print(label, " ".join(f"{value:.6f}" for value in row))
