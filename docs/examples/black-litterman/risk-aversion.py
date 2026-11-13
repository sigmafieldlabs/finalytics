import finalytics as fl

market_returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
estimate = fl.opt.implied_risk_aversion(
    market_returns, periods_per_year=12, risk_free_rate=0.02
)
print(f"Implied risk aversion: {estimate:.6f}")
