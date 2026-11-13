import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    instruments="id,kind,currency\nSAP,equity,EUR\n",
    transactions=(
        "date,type,instrument,quantity,price,amount,currency\n"
        "2026-06-30,deposit,,,,2000,USD\n"
        "2026-06-30,transfer_in,SAP,10,,,\n"
    ),
    prices="date,SAP\n2026-06-30,200\n",
    fx="date,EUR/USD\n2026-06-30,1.10\n",
)
position = portfolio.valuation().positions[0]
print("Local:", position.market_value_local)
print("Base:", position.market_value_base)
