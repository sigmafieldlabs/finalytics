import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices())
result = assets.evaluate(weights={"US Equity": 0.60, "Bonds": 0.40})
print("Expected return:", f"{result.results.expected_return:.6f}")
print("Volatility:", f"{result.results.volatility:.6f}")
print("Sharpe ratio:", f"{result.results.sharpe_ratio:.6f}")
