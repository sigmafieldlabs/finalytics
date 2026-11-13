---
description: Choose the weights of a set of assets with any of the 15 methods, under the limits you set, and read what the result holds.
learn: portfolio/1.4
---

# Optimise a portfolio

`optimise` chooses the weights of a set of assets by one of 15 methods, under the limits you
set, and returns them with a report on what they are expected to deliver and where their risk
comes from.

## The task in one example

The asset-class dataset holds 30 years of month-end levels of six asset-class indices. With
T-bills as cash, these are the weights with the largest Sharpe ratio, at most 40% in any asset
class:

[optimise.py](../../examples/optimise-a-portfolio/optimise.py)

## Read the result

The weights put 40.0% in bonds, at the limit, 38.3% in US equity and 21.7% in gold, and leave
out international equity and real estate. They are expected to return 7.0% a year at a
volatility of 8.3%: a Sharpe ratio of 0.57.

The weights and the risk tell different stories. US equity is 38.3% of the money but 64.4% of the
risk, its share of the portfolio's variance, while bonds are 40.0% of the money and 11.2% of the
risk. The three assets held count as 2.8 assets in effect.

> [!NOTE]
> The line "On the history it was fitted to" scores the weights on the returns they were chosen
> from, so it flatters them. A backtest tests weights on returns they had not seen.

## Choose a method

Maximum Sharpe is the default, not a universal answer. Minimum variance seeks the least risky
mix; target return and target volatility turn a required return or risk budget into a
constraint; risk-parity methods allocate by risk rather than expected return. Equal weight and
inverse volatility are useful baselines.

`methods()` describes every method, its parameters and the limits it accepts. Use it to build a
form or check what is available instead of duplicating that list in your application:

[methods.py](../../examples/optimise-a-portfolio/methods.py)

Each entry also describes defaults, valid ranges and whether the method resamples or works
directly from return scenarios. Methods that are fully invested and long only by construction
do not accept weight limits.

`optimise` uses long-only weights by default. The lower-level `Problem` API can model negative
weights for supported objectives when explicitly configured, but the transaction ledger and
rebalancing cannot currently execute or represent those short positions.

## Change the objective

A target-volatility optimisation seeks the largest expected return within an annual volatility
limit. Here the limit is 10.0%, with the same 40% cap on each asset:

[target-volatility.py](../../examples/optimise-a-portfolio/target-volatility.py)

The report names the method, target and solver status alongside the weights. Change `target`
to express your risk budget, or choose another method rather than treating maximum Sharpe as
a setting that fits every problem.

## Change the estimates

By default, expected returns are historical means and covariance is estimated with
Ledoit–Wolf shrinkage. `expected_returns` replaces the mean for the assets you name while
leaving the others and the covariance estimate in place. This example gives US equity an
expected return of 4.0% a year:

[estimates.py](../../examples/optimise-a-portfolio/estimates.py)

The report's estimates section records that change. Expected returns can dominate optimised
weights, so compare reasonable alternatives and test the allocation out of sample rather than
precision-tuning one historical fit.

## Allow cash

Optimisation is fully invested in the risky assets unless you allow cash. With T-bills as the
risk-free asset, this minimum-variance example may hold up to 20.0% there:

[cash.py](../../examples/optimise-a-portfolio/cash.py)

Without a named risk-free asset, the cash appears as `Cash` and earns the risk-free rate.
Maximum Sharpe and maximum diversification do not take cash limits because risk-free cash
leaves their ratios unchanged.

To preserve cash already held instead, provide `current_weights` and use `keep_cash`. The
remaining assets are optimised while the specified cash stays at its current weight.

## When no weights satisfy the limits

An unattainable target is a result, not an exception. No portfolio with at most 40% in each
asset can meet this required expected return of 20.0% a year:

[infeasible.py](../../examples/optimise-a-portfolio/infeasible.py)

Check `result.results.status` before using `result.results.weights`: an infeasible result has
no weights. Relax a limit or reconsider the objective. A status of `nearly_optimal` does have
weights, but its solution's largest constraint violation deserves inspection.

Invalid input is different: an unknown option, a missing target or an option that the chosen
method does not use raises a validation error naming the input to fix.

## Next steps

Add `current_weights` to see trades and constrain turnover, or `benchmark_weights` to measure
tracking error. The [optimisation reference](../../methods/optimisation.md) covers group
limits, trade limits, resampling and the solver statuses.

An optimised allocation is a hypothesis about the future. Use a
[backtest](../../methods/backtests.md) to refit it on past windows and evaluate it on returns
it had not seen before using it to rebalance a real portfolio.
