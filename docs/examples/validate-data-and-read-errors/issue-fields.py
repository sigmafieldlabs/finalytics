import finalytics as fl

try:
    fl.PortfolioData.from_csv(
        transactions="date,type,amount,currency\nnot-a-date,deposit,1000,USD\n"
    )
except fl.ValidationError as error:
    for issue in error.issues:
        print(issue.severity, issue.path)
        print(issue.message)
