---
description: Read every model's definitions, assumptions, conventions and independent numerical validation.
---

# Check the methods and validation

The methods pages define what Finalytics computes. They are the place to check an estimator, timing convention, formula or assumption before relying on a result; the task guides show how to call it.

## Portfolio records and measurement

| Method | What it defines |
| --- | --- |
| [Tax lots](../../methods/tax-lots.md) | Exact ledger lots, relief methods, corporate actions and realised gains |
| [Valuation](../../methods/valuation.md) | Positions, cash, base-currency value and reconciliation through time |
| [Returns](../../methods/returns.md) | Time-weighted, money-weighted and Modified Dietz returns and periods |
| [Contribution](../../methods/contribution.md) | Holding and group contributions, including currency effects |
| [Attribution](../../methods/attribution.md) | Brinson–Fachler and Karnosky–Singer attribution and linking |
| [Households](../../methods/households.md) | Combined accounts, owners, wrappers and household reporting |

## Risk and estimates

| Method | What it defines |
| --- | --- |
| [Statistics](../../methods/statistics.md) | Return and risk statistics, relative measures and drawdowns |
| [Tail risk](../../methods/tail-risk.md) | Historical, parametric and Cornish–Fisher VaR and expected shortfall |
| [Covariance](../../methods/covariance.md) | Covariance and expected-return estimators and annualisation |
| [Risk decomposition](../../methods/risk-decomposition.md) | Euler shares, tracking-error and shortfall decompositions |
| [Factor models](../../methods/factor-models.md) | Regression, statistical and fundamental factor models |
| [Scenarios](../../methods/scenarios.md) | Shock and historical scenarios, proxies and covariance stress |

## Allocation and testing

| Method | What it defines |
| --- | --- |
| [Optimisation](../../methods/optimisation.md) | Objectives, estimators, constraints, frontiers and solver conventions |
| [Robust optimisation](../../methods/robust-optimisation.md) | Uncertainty sets, robust objectives and resampling |
| [Black–Litterman](../../methods/black-litterman.md) | Prior equilibrium returns, views, confidence and posterior estimates |
| [Backtests](../../methods/backtests.md) | Rebalancing, costs, holdout and walk-forward validation and money flows |
| [Rebalancing](../../methods/rebalancing.md) | Trades from targets, lots, tax, fees, cash and several accounts |
| [Planning](../../methods/planning.md) | Goal simulations, inflation, saving, spending and alternative mixes |

## Screening and presentation

| Method | What it defines |
| --- | --- |
| [Screening](../../methods/screening.md) | Every measure, formula, rule, ranking and coverage convention |
| [Reports](../../methods/reports.md) | The six report parts, provenance, tables and presentation options |

## Independent validation

[Methods and validation conventions](../../methods/README.md) defines the decimal, sign, rate, annualisation and error conventions shared by every model.

Each page names its reference implementation or independent calculation. The repository's `scripts/reference/` programs recompute the models with tools such as NumPy, pandas, SciPy, scikit-learn and CVXPY, and the test suite states the numerical tolerance for each comparison.

A passing test supports the implementation of the stated model; it does not make the model's assumptions true for every market or portfolio. Read the conventions and limits alongside the numerical agreement.
