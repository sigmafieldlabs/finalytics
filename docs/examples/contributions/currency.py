import finalytics as fl

for item in fl.datasets.portfolio().performance().contributions():
    if item.holding == "EXSA":
        print("Local:", f"{item.return_local:.6f}")
        print("Base:", f"{item.return_base:.6f}")
        print("Currency effect:", f"{item.currency_effect:.6f}")
