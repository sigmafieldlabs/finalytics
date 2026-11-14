import finalytics as fl

portfolio = fl.datasets.portfolio()
options = fl.datasets.report_options()
result = portfolio.report(
    options,
    start="2025-01-01",
    name="Attribution and scenarios since 2025",
)

print(result)
result.show()
