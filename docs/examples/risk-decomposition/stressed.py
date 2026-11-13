import finalytics as fl

covariance = fl.risk.matrix(
    [[0.04, 0.01], [0.01, 0.09]], ["equity", "bonds"]
)
stressed = fl.risk.stressed_covariance(
    covariance, volatility_scale=1.5, correlation_shift=0.5
)
for name, matrix in (("Before", covariance), ("After", stressed)):
    result = fl.risk.risk_decomposition([0.6, 0.4], matrix)
    print(f"{name}: volatility={result.volatility:.6f}")
    for item in result.contributions:
        print(f"  {item.label}: share={item.share:.6f}")
