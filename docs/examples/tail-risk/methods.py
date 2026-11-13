import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
for method in ("historical", "parametric", "cornish_fisher"):
    result = fl.risk.tail_risk(returns, confidence=0.90, method=method)
    print(
        f"{method}: VaR={result.value_at_risk:.6f} "
        f"ES={result.expected_shortfall:.6f}"
    )
