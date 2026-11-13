import { karnoskySinger } from "finalytics";

const period = {
  start: "2026-01-01", end: "2026-12-31",
  segments: [
    { name: "US", currency: "USD", portfolio_weight: 0.5,
      portfolio_return: 0.03, benchmark_weight: 0.6, benchmark_return: 0.035 },
    { name: "Europe", currency: "EUR", portfolio_weight: 0.5,
      portfolio_return: 0.05, benchmark_weight: 0.4, benchmark_return: 0.04 },
  ],
  currencies: [
    { currency: "USD", spot_return: 0.0, cash_return: 0.04 },
    { currency: "EUR", spot_return: 0.03, cash_return: 0.025 },
  ],
  portfolio_exposures: { USD: 0.75, EUR: 0.25 },
};
const result = karnoskySinger([period]);
for (const currency of result.currencies) {
  console.log(currency.currency, currency.allocation.toFixed(6));
}
console.log("Residual:", result.residual.toFixed(6));
