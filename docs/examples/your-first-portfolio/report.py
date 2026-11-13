import finalytics as fl

portfolio = fl.datasets.portfolio()
report = portfolio.report(
    benchmark="BENCH", group_by=["asset_class", "region"]
)

print(report)
report.show()
