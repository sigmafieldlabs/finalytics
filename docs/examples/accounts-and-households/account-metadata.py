import finalytics as fl

portfolio = fl.datasets.portfolio()
for account in portfolio.data.accounts:
    import json
    print(account.id, account.name, str(account.taxable).lower(),
          json.dumps(account.classifications, separators=(",", ":")))
