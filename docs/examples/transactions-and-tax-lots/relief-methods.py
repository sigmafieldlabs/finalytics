import finalytics as fl

base = (
    "id,account,trade_date,type,instrument,quantity,price,fees,"
    "amount,currency,lot_method,lots\nt1,main,2026-01-02,deposit"
    ",,,,,10000,USD,,\nt2,main,2026-01-05,buy,AAPL,10,150,5,,,,\n"
    "t3,main,2026-03-02,buy,AAPL,10,200,5,,,,\n"
)
for method in ("fifo", "lifo", "highest_cost", "lowest_cost"):
    sale = (
        "id,account,trade_date,type,instrument,quantity,price,fees,"
        "amount,currency,lot_method,lots\n"
        f"t4,main,2026-06-01,sell,AAPL,5,190,0,,,{method},\n"
    )
    lines = base.splitlines()
    portfolio = fl.Portfolio.from_csv(
        transactions="\n".join([lines[0], sale.splitlines()[1], *lines[1:]])
    )
    lots = ",".join(g.lot for g in portfolio.realised_gains())
    print(method, lots)
