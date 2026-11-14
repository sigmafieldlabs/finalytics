import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.holdings_report(group_by=["asset_class", "region"])
print(result)

source = result.provenance
print(f"Report: {source.report}")
print(f"As of: {source.as_of}")
print(f"Base currency: {source.base_currency}")
print(f"Source digest: {source.source.digest}")
