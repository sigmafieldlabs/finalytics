import finalytics as fl

original = fl.datasets.portfolio().data
restored = fl.PortfolioData.from_json(original.to_json())
print("Equal records:", str(original == restored).lower())
print("Instruments:", len(restored.instruments))
