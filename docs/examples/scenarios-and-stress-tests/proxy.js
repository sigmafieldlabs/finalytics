import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const result = portfolio.historicalScenario({
  start: "2025-02-19",
  end: "2025-04-08",
  proxies: { EXSA: "VTI" },
});
for (const holding of result.holdings) {
  if (["VTI", "EXSA"].includes(holding.holding)) {
    console.log(
      `${holding.holding}: local=${holding.return_local.toFixed(6)}, ` +
        `currency=${holding.currency_return.toFixed(6)}, ` +
        `base=${holding.return_base.toFixed(6)}`,
    );
  }
}
