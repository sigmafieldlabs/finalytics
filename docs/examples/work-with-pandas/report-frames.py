import pandas as pd
import finalytics as fl

report = fl.datasets.portfolio().performance_report(benchmark="BENCH")
for table in fl.reports.tables(report):
    frame = pd.DataFrame(
        table.rows, columns=[column.label for column in table.columns]
    )
    print(f"{table.name}: {frame.shape[0]} rows, {frame.shape[1]} columns")
    print(frame.head(2).to_string(index=False))
    print()
