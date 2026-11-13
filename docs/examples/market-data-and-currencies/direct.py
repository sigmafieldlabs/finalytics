import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions="date,type,amount,currency\n2026-06-30,deposit,100,USD\n",
    fx="date,EUR/USD\n2026-06-30,1.10\n",
)
rate = portfolio.market.fx_rate("EUR", "USD", "2026-06-30")
print("EUR/USD:", f"{rate.value:.2f}")
print("100 EUR in USD:", f"{100 * rate.value:.2f}")
