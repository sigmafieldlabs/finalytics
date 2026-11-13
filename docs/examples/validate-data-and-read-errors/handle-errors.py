import finalytics as fl

try:
    fl.Assets(fl.datasets.asset_class_prices()).optimise(
        method="target_return"
    )
except fl.ValidationError as error:
    print(type(error).__name__)
    print("Issues:", len(error.issues))
