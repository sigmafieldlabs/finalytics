import numpy as np
import finalytics as fl

returns = np.array([
    [0.0176, -0.0114, 0.0205, -0.0052, 0.0097,
     -0.0133, 0.0078, 0.0016, -0.0058, 0.0120],
    [0.0110, -0.0060, 0.0150, -0.0060, 0.0100,
     -0.0110, 0.0070, 0.0010, -0.0060, 0.0110],
    [0.0084, -0.0076, 0.0125, -0.0018, 0.0063,
     -0.0087, 0.0062, 0.0014, -0.0052, 0.0080],
    [0.0034, -0.0016, 0.0020, 0.0012, 0.0008,
     -0.0012, -0.0008, 0.0014, -0.0012, 0.0010],
]).T
market = np.array([
    0.012, -0.008, 0.015, -0.004, 0.009,
    -0.011, 0.006, 0.002, -0.006, 0.010,
]).reshape(-1, 1)
estimate = fl.risk.regression_model(
    returns,
    market,
    labels=["tech", "industrials", "utilities", "bonds"],
    factor_labels=["market"],
)
print(f"Method: {estimate.method}")
print(f"Observations: {estimate.observations}")
for fit, exposures in zip(estimate.fit, estimate.model.exposures):
    print(
        f"{fit.asset}: beta={exposures[0]:.6f} "
        f"alpha={fit.intercept:.6f} R2={fit.r_squared:.6f}"
    )
