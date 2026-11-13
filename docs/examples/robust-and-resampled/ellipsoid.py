import finalytics as fl

labels = ["us equity", "intl equity", "bonds"]
covariance = [
    [0.0256, 0.02304, 0.0008],
    [0.02304, 0.0324, 0.0009],
    [0.0008, 0.0009, 0.0025],
]
problem = fl.opt.Problem.from_covariance(
    covariance,
    labels=labels,
    expected_returns=[0.07, 0.08, 0.03],
)
objective = fl.opt.RobustMeanVariance(
    risk_aversion=4.0,
    uncertainty=fl.opt.Ellipsoid(size=0.5),
)
solution = problem.solve(objective)
print(f"Status: {solution.status}")
for label, weight in zip(solution.labels, solution.weights):
    print(f"{label}: {weight:.6f}")
print(f"Objective: {solution.objective_value:.6f}")
