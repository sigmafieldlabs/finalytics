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
exposures = np.array([
    [1.0, 0.0], [1.0, 0.0],
    [0.0, 1.0], [0.0, 1.0],
])
estimate = fl.risk.fundamental_model(
    returns,
    exposures,
    labels=["tech", "industrials", "utilities", "bonds"],
    factors=["cyclical", "defensive"],
)
print(f"Method: {estimate.method}")
for label, column in zip(
    estimate.model.factor_covariance.labels,
    estimate.factor_returns,
):
    print(
        f"{label}: first={column[0]:.6f} last={column[-1]:.6f}"
    )
for fit in estimate.fit:
    print(f"{fit.asset}: R2={fit.r_squared:.6f}")
