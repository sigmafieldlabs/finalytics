# Methods and validation

These pages define each calculation in Finalytics: the formula, the conventions chosen where
practice differs, the inputs it accepts, the references it follows, and how it is checked
against independent implementations. The examples are in Python and are run as tests, so their
outputs are the engine's own.

| Page | Covers |
| --- | --- |
| [Tax lots and cost basis](tax-lots.md) | Lot relief methods, corporate actions, realised gains in two currencies |
| [Valuation and FX rates](valuation.md) | Prices and rates found directly, inverted or crossed; carrying values forward |
| [Returns](returns.md) | Time-weighted, money-weighted and Modified Dietz returns; periods and annualisation; excess returns |
| [Contribution to return](contribution.md) | Each holding's share of the return, its own return and the currency effect |
| [Performance attribution](attribution.md) | Brinson–Fachler allocation, selection and interaction; linking periods; Karnosky–Singer currency attribution |
| [Households](households.md) | Accounts and their groups; performance by account and by group; contribution by account to the consolidated return |
| [Return statistics and drawdowns](statistics.md) | Volatility, Sharpe, Sortino and Calmar ratios, moments, statistics against a benchmark, drawdowns |
| [Value at risk and expected shortfall](tail-risk.md) | Historical, parametric and Cornish–Fisher estimates |
| [Covariance and mean returns](covariance.md) | Sample, exponentially weighted, shrinkage, statistical, denoised and nonlinear covariance estimators; historical, James–Stein and exponentially weighted means |
| [Factor models](factor-models.md) | Regression, statistical and fundamental models; risk by factor; optimisation in factor form with exposure limits |
| [Risk decomposition](risk-decomposition.md) | Marginal and component risk, ex-ante tracking error, risk by group |
| [Portfolio optimisation](optimisation.md) | Each objective and constraint, its formulation, and how solutions are reported |
| [Black–Litterman](black-litterman.md) | Equilibrium returns, views and their uncertainty, the posterior returns and covariance |
| [Robust and resampled optimisation](robust-optimisation.md) | Worst-case mean–variance over ellipsoids and boxes; Michaud resampling |
| [Backtests and comparisons](backtests.md) | Weights held and rebalanced on a schedule, trading costs and cash; in-sample, holdout and walk-forward validation; the comparison and backtest reports |
| [Goal planning](planning.md) | Plans of saving, spending and investing towards a goal; simulated markets; the chance, the changes that reach it, and the plan when markets do worse |
| [Screening](screening.md) | Measures of many securities on their own histories: returns and risk over calendar windows, measures against a benchmark, technical indicators, income and volume; rules on measures and fields |
| [Rebalancing](rebalancing.md) | Trades from current holdings to target weights, with lot selection |
| [Scenarios](scenarios.md) | Shocks to instruments, groups, currencies and factors; historical replay; the effect on value and on risk, with stressed correlations |
| [Reports](reports.md) | The holdings, overview, performance, risk, attribution and scenario reports; provenance; shapes for charts; tables for export |

## Conventions used throughout

- **Decimals and floats.** Quantities, prices, amounts and cost bases are exact decimals; they
  are written as strings, such as `"201.50"`, so that no language rounds them. Analytics
  (returns, statistics, optimisation) use 64-bit floating point.
- **Signs.** A return of `-0.25` is a fall of 25%. Drawdowns are negative returns. Value at risk
  and expected shortfall are losses, reported as positive numbers.
- **Rates.** Returns and rates are fractions, not percentages: `0.03` is 3%.
- **Annualisation.** A period's time-weighted return is annualised with 365.25 days a year, and
  only when the period spans at least 365 days. The money-weighted return is an annual rate by
  construction and counts time as actual/365. Statistics of a series of returns annualise with
  the number of periods a year (252 for daily returns by default).
- **Errors.** Invalid input is reported with every problem at once, each with a path to where it
  is (`transactions[14].quantity`, `returns[3]`). A computation that is not defined for valid
  input, such as a ratio over a dispersion of zero, returns no value rather than a NaN.

## How the checks work

Each page's "Validation" section names an independent implementation and a tolerance. Scripts in
[`scripts/reference/`](../../scripts/reference) produce the reference values with those
implementations, and the values are committed as fixtures. The Rust tests compare against them,
and so do the Python and JavaScript tests, so every language is held to the same numbers.
