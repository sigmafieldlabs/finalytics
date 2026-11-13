import finalytics as fl

try:
    fl.Portfolio.from_csv(
        transactions=(
            "date,type,instrument,quantity,price,amount,currency\n"
            "2026-01-02,deposit,,,,1000,USD\n"
            "2026-01-02,sell,VOO,2,500,,\n"
        )
    )
except fl.ValidationError as error:
    for issue in error.issues:
        print(issue.path, "|", issue.message)
