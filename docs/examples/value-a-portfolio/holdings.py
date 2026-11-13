import finalytics as fl

holdings = fl.datasets.portfolio().holdings("2026-10-30")
for position in holdings.positions:
    print(position.account, position.instrument, position.quantity,
          position.cost_base)
