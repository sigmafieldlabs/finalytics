import finalytics as fl

assets = fl.Assets(
    fl.datasets.asset_class_prices(), risk_free_asset="T-Bills"
)
for item in assets.statistics():
    stats = item.statistics
    tail = item.tail_risk
    print(
        f"{item.asset}: return={stats.annualised_return:.6f} "
        f"volatility={stats.volatility:.6f} "
        f"VaR={tail.value_at_risk:.6f} ES={tail.expected_shortfall:.6f}"
    )
