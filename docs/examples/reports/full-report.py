import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.report(
    fl.datasets.report_options(), name="Morgan family portfolio"
)

print(result)
result.show()
