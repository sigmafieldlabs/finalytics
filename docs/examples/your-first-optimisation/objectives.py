import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
for options in (
    {"method": "minimum_variance"},
    {"method": "target_return", "target": 0.06},
    {"method": "target_volatility", "target": 0.08},
):
    result = assets.optimise(**options)
    print(options["method"], f"{result.results.expected_return:.6f}",
          f"{result.results.volatility:.6f}")
