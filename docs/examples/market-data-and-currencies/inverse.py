import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions="date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
    fx="date,USD/EUR\n2026-06-30,0.80\n",
)
print(portfolio.market.fx_rate("EUR", "USD", "2026-06-30").value)
