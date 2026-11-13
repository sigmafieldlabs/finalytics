import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices(), {
  risk_free_asset: "T-Bills",
});
for (const item of assets.statistics()) {
  const stats = item.statistics;
  const tail = item.tail_risk;
  console.log(
    `${item.asset}: return=${stats.annualised_return.toFixed(6)} ` +
    `volatility=${stats.volatility.toFixed(6)} ` +
    `VaR=${tail.value_at_risk.toFixed(6)} ` +
    `ES=${tail.expected_shortfall.toFixed(6)}`,
  );
}
