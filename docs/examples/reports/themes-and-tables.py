import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.report(benchmark="BENCH")
print(result)
result.show(theme="dark")

for table in fl.reports.tables(result):
    print(f"Table: {table.name} ({len(table.rows)} rows)")
