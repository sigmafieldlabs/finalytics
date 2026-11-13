import finalytics as fl

valuation = fl.datasets.portfolio().valuation()
print("Date:", valuation.date)
print("Positions:", len(valuation.positions))
print("Cash balances:", len(valuation.cash))
print("Total:", valuation.total_base)
