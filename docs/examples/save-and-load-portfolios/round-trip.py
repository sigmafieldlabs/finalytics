import finalytics as fl

original = fl.datasets.portfolio()
restored = fl.Portfolio.read(original.to_json())
equal = original.valuation().total_base == restored.valuation().total_base
print("Equal values:", str(equal).lower())
print("Transactions:", len(restored.data.transactions))
