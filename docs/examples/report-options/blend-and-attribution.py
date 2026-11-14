import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.report(
    benchmark={"VTI": 0.55, "BND": 0.35, "GLD": 0.1},
    attribution="asset_class",
    group_by=["asset_class"],
    name="Against a blended benchmark",
)

print(result)
result.show()
