import finalytics as fl

data = fl.PortfolioData.from_csv(
    transactions=(
        "date,type,amount,currency\n"
        "2026-01-02,deposit,1000,USD\n"
    )
)
print("Issues:", data.validate())
