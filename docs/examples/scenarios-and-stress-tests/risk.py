import finalytics as fl

portfolio = fl.datasets.portfolio()
labels = ["VTI", "EXSA", "BND", "GLD", "VNQ", "EUR cash"]
covariance = fl.risk.matrix(
    [
        [0.0400, 0, 0, 0, 0, 0],
        [0, 0.0625, 0, 0, 0, 0],
        [0, 0, 0.0036, 0, 0, 0],
        [0, 0, 0, 0.0225, 0, 0],
        [0, 0, 0, 0, 0.0400, 0],
        [0, 0, 0, 0, 0, 0.0001],
    ],
    labels,
)
result = portfolio.shock_scenario(
    groups={("asset_class", "Equity"): -0.20},
    risk=covariance,
    confidence=0.99,
    stress=fl.risk.CorrelationStress(
        volatility_scale=1.5,
        correlation_shift=0.5,
    ),
)
for name, risk in (
    ("Before", result.risk_before),
    ("After", result.risk_after),
):
    print(f"{name}: volatility={risk.volatility:.6f}")
    print(f"{name}: value at risk={risk.value_at_risk_base:.2f}")
