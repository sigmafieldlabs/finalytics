import finalytics as fl

try:
    fl.Assets(fl.datasets.asset_class_prices()).optimise(
        method="target_return", max_weight=1.5
    )
except fl.ValidationError as error:
    for issue in error.issues:
        print(issue.path, "|", issue.message)
