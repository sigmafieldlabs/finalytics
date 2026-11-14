import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.report(
    benchmark="BENCH",
    group_by=["asset_class", "region"],
    confidences=[0.9, 0.95, 0.99],
    rolling_window=63,
    histogram_bins=20,
    top=5,
    name="Risk presentation options",
)

print(result)
result.show()
