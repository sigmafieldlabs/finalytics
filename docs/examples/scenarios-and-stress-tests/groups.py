import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.shock_scenario(
    groups={
        ("asset_class", "Equity"): -0.25,
        ("asset_class", "Real assets"): -0.10,
        ("asset_class", "Fixed income"): 0.03,
    },
    group_by=["asset_class"],
)
print(f"Value before: {result.value_before:.2f}")
print(f"Value after: {result.value_after:.2f}")
print(f"Portfolio return: {result.portfolio_return:.6f}")
for group in result.groups[0].groups:
    print(f"{group.group}: contribution={group.contribution:.6f}")
