import finalytics as fl

household = fl.datasets.portfolio().household(group_by=["owner"])
print("Household:", f"{household.household.time_weighted:.6f}")
for account in household.accounts:
    print(account.name, f"{account.time_weighted:.6f}",
          f"{account.contribution:.6f}")
