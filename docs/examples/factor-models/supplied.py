import finalytics as fl

model = fl.risk.factor_model(
    [[1.0, 0.0], [1.2, 0.0], [0.0, -6.0]],
    [[0.04, -0.0016], [-0.0016, 0.0001]],
    [0.01, 0.02, 0.0001],
    assets=["SPY", "SAP", "BOND"],
    factors=["equity", "rates"],
)
print(f"Assets: {', '.join(model.assets)}")
factors = ", ".join(model.factor_covariance.labels)
print(f"Factors: {factors}")
implied = fl.risk.implied_covariance(model)
for label, row in zip(implied.labels, fl.risk.to_array(implied)):
    print(label, " ".join(f"{value:.6f}" for value in row))
