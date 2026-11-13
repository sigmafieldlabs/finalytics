import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.shockScenario({
  groups: [
    { scheme: "asset_class", value: "Equity", shock: -0.20 },
  ],
  currencies: { EUR: -0.10 },
});
for (const holding of result.holdings) {
  if (holding.return_local !== 0 || holding.currency_return !== 0) {
    console.log(
      `${holding.holding}: local=${holding.return_local.toFixed(6)}, ` +
        `currency=${holding.currency_return.toFixed(6)}, ` +
        `base=${holding.return_base.toFixed(6)}`,
    );
  }
}
