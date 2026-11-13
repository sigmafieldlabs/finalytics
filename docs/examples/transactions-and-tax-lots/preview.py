import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions=(
        "id,account,trade_date,type,instrument,quantity,price,fees,"
        "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit"
        ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n"
        "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n"
    )
)
for method in ("fifo", "highest_cost"):
    lots = portfolio.lots_for_sale(
        "2026-06-30", "main", "AAPL", 5, method=method
    )
    print(method, ",".join(f"{lot.lot}:{lot.quantity}" for lot in lots))
print("Open lots:", len(portfolio.holdings().lots))
