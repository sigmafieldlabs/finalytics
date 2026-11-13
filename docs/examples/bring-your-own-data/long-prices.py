import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions=(
        "date,type,instrument,quantity,price,amount,currency\n"
        "2026-01-02,deposit,,,,1000,USD\n"
        "2026-01-02,buy,VOO,1,500,,\n"
    ),
    prices=(
        "date,instrument,price\n"
        "2026-01-02,VOO,500\n"
        "2026-01-05,VOO,505\n"
    ),
)
print(portfolio.valuation().total_base)
