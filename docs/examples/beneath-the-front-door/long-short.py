import finalytics as fl

labels = ["equity", "credit", "bonds"]
covariance = [
    [0.0400, 0.0060, 0.0020],
    [0.0060, 0.0144, 0.0015],
    [0.0020, 0.0015, 0.0036],
]
constraints = fl.opt.Constraints(
    bounds=fl.opt.Bound(min=-0.5, max=1.0),
    leverage=1.5,
)
problem = fl.opt.Problem.from_covariance(
    covariance,
    labels=labels,
    expected_returns=[0.07, 0.045, 0.03],
    constraints=constraints,
)
solution = problem.solve(fl.opt.TargetReturn(target=0.07))
print(f"Status: {solution.status}")
for label, weight in zip(solution.labels, solution.weights):
    print(f"{label}: {weight:.6f}")
print(f"Largest violation: {solution.max_violation:.9f}")
print(f"Gross exposure: {sum(abs(w) for w in solution.weights):.6f}")
