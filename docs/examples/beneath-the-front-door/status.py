import finalytics as fl

labels = ["equity", "credit", "bonds"]
covariance = [
    [0.0400, 0.0060, 0.0020],
    [0.0060, 0.0144, 0.0015],
    [0.0020, 0.0015, 0.0036],
]
constraints = fl.opt.Constraints(
    bounds=fl.opt.Bound(min=0.0, max=0.2),
)
problem = fl.opt.Problem.from_covariance(
    covariance,
    labels=labels,
    expected_returns=[0.07, 0.045, 0.03],
    constraints=constraints,
)
solution = problem.solve(fl.opt.MinimumVariance())
print(f"Status: {solution.status}")
if solution.weights is None:
    print("Weights: none")
else:
    for label, weight in zip(solution.labels, solution.weights):
        print(f"{label}: {weight:.6f}")
