import finalytics as fl

with open("transactions.csv", encoding="utf-8") as file:
    transactions = file.read()
with open("prices.csv", encoding="utf-8") as file:
    prices = file.read()
with open("fx.csv", encoding="utf-8") as file:
    fx = file.read()

portfolio = fl.Portfolio.from_csv(
    transactions=transactions, prices=prices, fx=fx,
)
print("Transactions:", len(portfolio.data.transactions))
print("Value:", portfolio.valuation().total_base)
