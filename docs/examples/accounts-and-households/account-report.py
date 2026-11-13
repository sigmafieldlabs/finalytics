import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.report(accounts=["brokerage"])
print(result, end="")
result.show()
