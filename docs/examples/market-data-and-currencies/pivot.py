import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions="date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
    fx=(
        "date,EUR/USD,GBP/USD\n"
        "2026-06-30,1.10,1.25\n"
    ),
)
rate = portfolio.market.fx_rate("EUR", "GBP", "2026-06-30")
print("Pivot:", portfolio.market.pivot)
print("EUR/GBP:", rate.value)
