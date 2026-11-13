from pathlib import Path
import finalytics as fl

path = Path("saved-portfolio.json")
path.write_text(fl.datasets.portfolio().to_json(), encoding="utf-8")
restored = fl.Portfolio.read(path)
print("Restored:", restored.base_currency, len(restored.data.transactions))
path.unlink()
