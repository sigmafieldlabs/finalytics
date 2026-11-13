import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
for method in ("denoised", "nonlinear_shrinkage"):
    estimate = fl.risk.covariance(
        assets.returns, assets.labels, method=method
    )
    print(
        f"{method}: observations={estimate.observations} "
        f"first_variance={estimate.matrix.values[0]:.10f}"
    )
