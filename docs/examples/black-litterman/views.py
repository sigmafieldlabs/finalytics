import finalytics as fl

labels = ["us equity", "intl equity", "bonds"]
covariance = [
    [0.0256, 0.02304, 0.0008],
    [0.02304, 0.0324, 0.0009],
    [0.0008, 0.0009, 0.0025],
]
views = [
    fl.opt.View(
        portfolio={"intl equity": 1.0, "us equity": -1.0},
        expected_return=0.02,
        uncertainty=fl.opt.Confidence(confidence=0.6),
    ),
    fl.opt.View(
        portfolio={"bonds": 1.0},
        expected_return=0.04,
        uncertainty=fl.opt.Confidence(confidence=0.7),
    ),
]
estimate = fl.opt.black_litterman(
    covariance,
    views,
    labels=labels,
    market_weights=[0.55, 0.25, 0.20],
    risk_aversion=2.5,
)
for index, label in enumerate(estimate.labels):
    print(
        f"{label}: prior={estimate.prior_returns[index]:.6f}, "
        f"posterior={estimate.posterior_returns[index]:.6f}"
    )
for index, variance in enumerate(estimate.view_variances):
    print(f"View {index + 1} variance: {variance:.8f}")
