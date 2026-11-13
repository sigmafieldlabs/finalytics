import finalytics as fl

result = fl.datasets.portfolio().report(
    benchmark={"VTI": 0.60, "BND": 0.40}
)
print(result, end="")
result.show()
