import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    instruments=(
        "id,kind,currency,name,class.asset_class\n"
        "VOO,fund,USD,US stock fund,Equity\n"
    ),
    accounts=(
        "id,name,lot_method,taxable,class.owner\n"
        "main,Brokerage,fifo,false,Alice\n"
    ),
    transactions=(
        "id,account,date,type,instrument,quantity,price,amount,currency\n"
        "t1,main,2026-01-02,deposit,,,,1000,USD\n"
        "t2,main,2026-01-02,buy,VOO,1,500,,\n"
    ),
    prices="date,VOO\n2026-01-02,500\n2026-01-05,505\n",
)
print(portfolio.data.instruments[0].name)
import json
print(json.dumps(portfolio.data.instruments[0].classifications,
                 separators=(",", ":")))
print(portfolio.data.accounts[0].name,
      str(portfolio.data.accounts[0].taxable).lower())
