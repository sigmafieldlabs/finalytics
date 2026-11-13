import finalytics as fl

labels = ["equity", "credit", "bonds"]
covariance = [
    [0.0400, 0.0060, 0.0020],
    [0.0060, 0.0144, 0.0015],
    [0.0020, 0.0015, 0.0036],
]
constraints = fl.opt.Constraints(
    turnover=fl.opt.Turnover(
        current=[0.6, 0.2, 0.2],
        costs=[0.003, 0.003, 0.003],
        quadratic_costs=[0.05, 0.10, 0.20],
    ),
)
problem = fl.opt.Problem.from_covariance(
    covariance,
    labels=labels,
    expected_returns=[0.07, 0.045, 0.03],
    constraints=constraints,
)
solution = problem.solve(fl.opt.MeanVariance(risk_aversion=4.0))
print(f"Status: {solution.status}")
for label, weight in zip(solution.labels, solution.weights):
    print(f"{label}: {weight:.6f}")
print(f"Largest violation: {solution.max_violation:.9f}")
