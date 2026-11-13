import finalytics as fl

valuation = fl.datasets.portfolio().valuation()
positions = sum(p.market_value_base for p in valuation.positions)
cash = sum(c.value_base for c in valuation.cash)
print("Positions:", f"{positions:.2f}")
print("Cash:", f"{cash:.2f}")
print("Reconciles:", str(positions + cash == valuation.total_base).lower())
