import { Assets, datasets } from "finalytics";

      const assets = new Assets(datasets.assetClassPrices(), {
        risk_free_asset: "T-Bills",
      });
      const result = assets.optimise({
        method: "minimum_cvar",
        confidence: 0.95,
target: 0.06,
max_weight: 0.5,
      });

console.log(`${result}`);
result.show();
