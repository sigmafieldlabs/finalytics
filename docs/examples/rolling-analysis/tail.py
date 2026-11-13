import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
windows = fl.risk.rolling_tail_risk(
    returns, window=6, confidence=0.90, method="historical"
)
for index, result in enumerate(windows):
    print(
        f"Window {index}: VaR={result.value_at_risk:.6f} "
        f"ES={result.expected_shortfall:.6f}"
    )
