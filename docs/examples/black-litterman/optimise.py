import finalytics as fl

labels = ["us equity", "intl equity", "bonds"]
covariance = [
    [0.0256, 0.02304, 0.0008],
    [0.02304, 0.0324, 0.0009],
    [0.0008, 0.0009, 0.0025],
]
view = fl.opt.View(
    portfolio={"intl equity": 1.0, "us equity": -1.0},
    expected_return=0.02,
    uncertainty=fl.opt.Confidence(confidence=0.6),
)
estimate = fl.opt.black_litterman(
    covariance,
    [view],
    labels=labels,
    market_weights=[0.55, 0.25, 0.20],
    risk_aversion=2.5,
)
problem = fl.opt.Problem.from_black_litterman(estimate)
solution = problem.solve(fl.opt.MeanVariance(risk_aversion=2.5))
print(f"Status: {solution.status}")
for label, weight in zip(solution.labels, solution.weights):
    print(f"{label}: {weight:.6f}")
