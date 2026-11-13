import finalytics as fl

model = fl.risk.factor_model(
    [[1.0, 0.0], [0.8, 1.0], [0.0, 0.5]],
    [[0.04, 0.0], [0.0, 0.01]],
    [0.0025, 0.0025, 0.001],
    assets=["equity", "credit", "bonds"],
    factors=["market", "spread"],
)
constraints = fl.opt.Constraints(
    factor_exposures=[
        fl.opt.FactorExposureLimit(
            factor="market", min=0.4, max=0.6
        )
    ]
)
problem = fl.opt.Problem.from_factor_model(
    model, constraints=constraints
)
solution = problem.solve(fl.opt.MinimumVariance())
print(f"Status: {solution.status}")
for label, weight in zip(solution.labels, solution.weights):
    print(f"{label}: {weight:.6f}")
exposure = solution.weights[0] + 0.8 * solution.weights[1]
print(f"Market exposure: {exposure:.6f}")
