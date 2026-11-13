import math
import finalytics as fl

returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
]
monthly = fl.risk.tail_risk(returns, confidence=0.95, method="parametric")
root_time = math.sqrt(12)
print(f"Monthly VaR: {monthly.value_at_risk:.6f}")
print(f"Monthly ES: {monthly.expected_shortfall:.6f}")
print(f"Root-time annual VaR: {monthly.value_at_risk * root_time:.6f}")
print(f"Root-time annual ES: {monthly.expected_shortfall * root_time:.6f}")
