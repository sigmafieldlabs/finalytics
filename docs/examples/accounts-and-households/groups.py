import finalytics as fl

household = fl.datasets.portfolio().household(group_by=["owner", "wrapper"])
for scheme in household.groups:
    for group in scheme.groups:
        print(scheme.scheme, group.name, ",".join(group.accounts),
              f"{group.contribution:.6f}")
