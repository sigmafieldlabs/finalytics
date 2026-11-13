import finalytics as fl

assets = fl.Assets(
    "date,VOO,AGG\n"
    "2026-01-30,500,98\n"
    "2026-02-27,505,98.2\n"
    "2026-03-31,510,99.1\n"
)
print("Labels:", ", ".join(assets.labels))
print("Periods per year:", f"{assets.periods_per_year:g}")
