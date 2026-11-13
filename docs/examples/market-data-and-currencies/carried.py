import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions=(
        "date,type,instrument,quantity,price,amount,currency\n"
        "2026-06-30,deposit,,,,1000,USD\n"
        "2026-06-30,buy,VOO,1,500,,\n"
    ),
    prices="date,VOO\n2026-06-30,500\n",
)
found = portfolio.market.price("VOO", "2026-07-03")
print("Value:", found.value)
print("Observed:", found.date)
print("Carried:", str(found.carried).lower())
stale = portfolio.market.price("VOO", "2026-07-08") is None
print("Stale:", str(stale).lower())
