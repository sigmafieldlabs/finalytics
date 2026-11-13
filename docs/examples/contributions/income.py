import finalytics as fl

portfolio = fl.datasets.portfolio()
for item in portfolio.performance().contributions():
    if item.holding == "VTI":
        print("Holding:", item.holding)
        print("Contribution:", f"{item.contribution:.6f}")
        print("Own return:", f"{item.return_base:.6f}")
print("Events:", len(portfolio.events()))
