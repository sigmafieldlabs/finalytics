import finalytics as fl

portfolio = fl.datasets.portfolio()
series = portfolio.value_series()
print("Dates:", len(series.dates))
for date, value, flow in list(zip(series.dates, series.values,
                                  series.flows))[-3:]:
    print(date, value, flow)
