import finalytics as fl

portfolio = fl.datasets.portfolio()
result = portfolio.historical_scenario("2025-02-19", "2025-04-08")
print(f"Value before: {result.value_before:.2f}")
print(f"Value after: {result.value_after:.2f}")
print(f"Portfolio return: {result.portfolio_return:.6f}")
print(f"Path observations: {len(result.path.dates)}")
print(f"Path start: {result.path.values[0]:.2f}")
print(f"Path end: {result.path.values[-1]:.2f}")
drawdown = result.path.worst_drawdown
print(f"Worst drawdown: {drawdown.depth:.6f}")
print(f"Trough: {drawdown.trough}")
