import json
import finalytics as fl

document = json.loads(fl.datasets.portfolio().to_json())
print(document["format"])
print(document["format_version"])
print("Has market:", str("market" in document).lower())
