import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions=(
        "id,date,type,instrument,new_instrument,quantity,price,from"
        ",to,ratio,cost_allocation,cash_per_unit,amount,currency\nt1"
        ",2026-01-02,deposit,,,,,,,,,,10000,USD\nt2,2026-01-02,buy,A"
        "AA,,10,100,,,,,,,\nt3,2026-06-01,symbol_change,AAA,BBB,,,,,"
        ",,,,\n"
    )
)
for lot in portfolio.holdings().lots:
    print(lot.id, lot.instrument, lot.quantity, lot.cost_local)
for gain in portfolio.realised_gains():
    print("Gain:", gain.lot, gain.gain_local)
