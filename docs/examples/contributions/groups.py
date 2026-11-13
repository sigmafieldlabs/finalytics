import finalytics as fl

portfolio = fl.datasets.portfolio()
classes = {
    i.id: i.classifications["asset_class"]
    for i in portfolio.data.instruments
    if "asset_class" in i.classifications
}
def group(key):
    return "Cash" if key.endswith(" cash") else classes[key]
for item in fl.group_contributions(
    portfolio.performance().contributions(), group
):
    print(item.group, f"{item.contribution:.6f}")
