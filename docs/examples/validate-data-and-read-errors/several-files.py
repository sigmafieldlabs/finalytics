import finalytics as fl

try:
    fl.PortfolioData.from_csv(
        instruments="id,kind,currency\nAAPL,equity,usd\n",
        transactions=(
            "id,account,trade_date,type,instrument,quantity,price\n"
            "t1,main,2026-01-02,buy,AAPL,ten,150\n"
        ),
        base_currency="USD",
    )
except fl.ValidationError as error:
    for issue in error.issues:
        print(issue.path, "|", issue.message)
