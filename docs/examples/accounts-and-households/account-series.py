import finalytics as fl

portfolio = fl.datasets.portfolio()
series = portfolio.account_series()
print("Dates:", len(series.dates))
for line in series.lines:
    print(line.account, line.values[-1], line.flows[-1])
print("Household:", portfolio.value_series().values[-1])
