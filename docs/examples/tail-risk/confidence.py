import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
for confidence in (0.90, 0.95, 0.99):
    result = fl.risk.tail_risk(
        returns, confidence=confidence, method="historical"
    )
    print(
        f"Confidence {confidence:.2f}: VaR={result.value_at_risk:.6f} "
        f"ES={result.expected_shortfall:.6f}"
    )
