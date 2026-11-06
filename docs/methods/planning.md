# Goal planning

A plan saves, spends and invests towards one goal, and `assets.plan(...)` finds its chance of
meeting it in thousands of simulated markets drawn from the assets' monthly returns. The goal is
an amount by a date, such as a house deposit, or payments over a period, such as an income in
retirement; without a goal, the plan is a projection to a month. The planning report gives the
chance against the chance wanted, the value over time, the changes that would reach or keep that
chance, the plan when markets do worse, and other investment mixes on the same simulations.

## The plan

- **Goal.** `Target(amount, by)` is met when the plan is worth at least `amount` at the start of
  the month `by`. `Payments(amount, start, end, every)` pays `amount` every month, quarter or
  year from `start` to `end`, and is met when every payment is made in full. One plan has one
  goal.
- **Flows.** `Saving` pays money in and `Spending` takes it out, once (`on` a month) or
  regularly (`every` month, quarter or year, from `start` to `end`). A regular saving without an
  `end` runs until the goal: to the month before a target's date or before the first payment; a
  regular spending without one runs to the plan's last month. Spending is not scored; it is paid
  as far as the money goes.
- **Mix.** Fixed weights by asset, or a glide path: weights at some months, moving in a straight
  line between them and held before the first and after the last. The weights are rebalanced
  every month.
- **Start.** The plan starts in the month after the assets' last return unless `start` says
  otherwise, with `start_value`.

Each month, the value at its start is recorded; savings are paid in; spending and then any goal
payment are taken out, each as far as the money goes, whatever order the flows are listed in;
then the value grows by the month's growth factor. A payment the money cannot cover in full takes what is left and counts as missed.

## Today's money

Every amount is in today's money, because the returns are real: each month's growth factor is
the mix's return after the fee and inflation,

$$g_t = \max\!\left(\frac{1 + \sum_i w_{t,i} r_{t,i} - f/12}{1 + \pi_t},\ 0\right),$$

with $f$ the fee a year and $\pi_t$ the month's inflation: $(1 + \pi)^{1/12} - 1$ for a fixed
rate $\pi$ a year, or the price index's rise over the same month as the returns. The amounts stay
as given, so $4,000 in 2050 buys what $4,000 buys today. A flow with a `growth` rate is set in
money of the day instead: its amount in month $t$ is $a (1 + \gamma)^{t/12} / P_t$, with $P_t$
the price level since the start.

A price index (`fl.datasets.us_cpi()`, say) is matched to the assets' months by its last level
in each month; a month the index lacks is filled in between its neighbours at a steady rate, and
the history is cut to the months both cover.

## Simulating markets

The assets' returns are compounded into calendar months; a first or last month only partly
covered by daily or weekly returns is left out.

- **Bootstrap** (the default) is the stationary block bootstrap of Politis and Romano: each
  simulation replays runs of past months, all assets (and the price index) together, starting at
  a random month and continuing, with probability $1/b$ each month, from a new random month
  instead; the runs average $b$ months (12 by default) and wrap round at the end of the history.
  It keeps the months' correlations and some of their persistence.
- **Normal** draws each month's returns from a normal distribution with the assets' mean returns
  and covariance, estimated from the monthly returns by the assets' estimators.
- **Historical** replays every stretch of the history as long as the plan, in order, once each:
  there are as many simulations as stretches, and no draws at random.

`expected_returns` replace the history's average returns: the bootstrap shifts each asset's
months by the difference, and the normal model takes them as its means; the historical model,
which replays the history as it was, does not take them. A plan holds every simulated month at
once, so simulations times months is at most 24 million (10,000 simulations over 100 years). Each simulation has its
own random stream, from the seed and its number, so the results are the same on any number of
threads, and a search moves only the plan, never the markets.

## The chance and the status

The chance is the share of simulations that meet the goal. The strong, average and weak markets
are the 90th, 50th and 10th percentiles across the simulations, interpolated linearly. The
status compares the chance with the chance wanted, $c^*$ (0.8 by default, below 0.99), and the
chance from which there is room to spare, $c^{**}$ ($c^* + 0.10$, at most 0.99, by default):

| Status | Chance |
| --- | --- |
| Room to spare | at least $c^{**}$ |
| On track | at least $c^*$ |
| Nearly on track | up to 15 points below $c^*$ |
| Off track | more than 15 points below $c^*$ |

For payments, the money runs out, in a market, in the first payment month by which at least that
share of the simulations has missed a payment: 1 in 10 for the weak market, half for the average
and 9 in 10 for the strong. The share of the payments made is the money paid over the money planned.

## What would change the outcome

Each change is found on the same simulations, one at a time: below the chance wanted, the
smallest change that reaches it; at or above it, the largest change that keeps it. The search
runs on a grid of three significant figures of the current amount, and the answer is then
rounded to three significant figures of its own in the direction that keeps the chance (up for a
saving or a start value, down for the goal's amount).

- **Saving.** The largest regular saving, by its amount a month.
- **Start value.**
- **Amount.** For a target, the $1 - c^*$ quantile of the value at the date (the 20th
  percentile for a chance wanted of 80%), rounded down to the grid and lowered until it gives the
  chance wanted; for payments, each payment.
- **Date.** For a target, the earliest month by which the chance reaches $c^*$, saving until
  then (up to 20 years later, along the glide path if there is one), or the earliest month that
  keeps it; for payments, the earliest month of the first payment that reaches or keeps it,
  saving until then. The historical model has no months past the history to move a target
  later into.

## When markets do worse

- **Lower returns.** Every simulated month's growth is a twelfth of a percentage point lower:
  a point a year.
- **A crash.** The worst run of twelve months in the history for the plan's weights, by their
  return compounded, replaces the same twelve months of every simulation: the twelve before a
  target's date or the end of a projection, or the first twelve of payments. A plan with fewer
  months left takes the worst run of as many months as it has. Other mixes are tested with the
  same months of history, with their own weights.

## The mix's risk

The typical return is the median across the simulations of each one's growth, annualised, before
any money is paid in or out. The worst year in 20 is the 5th percentile of the twelve-month
returns, a year apart, across the simulations (none for a plan shorter than a year); the largest fall is the median of each
simulation's largest fall from a peak; the volatility is that of the mix's monthly returns in
the history, annualised by $\sqrt{12}$, before inflation.

## The report

The planning report, in this order: the plan's steps in time; the chance, with the status and the
value at the end in the strong, average and weak markets; 100 of the simulations, picked evenly
from the best outcome to the worst, with the average market over them; the changes; the plan
when markets do worse; the mix and the alternatives; and how the figures were worked out, with
disclosures that the projections are hypothetical and that the report is not advice. Amounts are
shown to three significant figures where they are estimates.

## Example

```python
>>> import finalytics as fl
>>> assets = fl.Assets(fl.datasets.asset_class_prices())
>>> plan = assets.plan(
...     fl.Target(amount=150_000, by="2030-06"),
...     start_value=60_000,
...     flows=[
...         fl.Saving(amount=2_500, every="month"),
...         fl.Spending(name="Car", amount=25_000, on="2028-03"),
...     ],
...     weights={"US Equity": 0.6, "Bonds": 0.4},
...     inflation=0.025,
...     fee=0.0025,
...     simulations=2_000,
... )
>>> r = plan.results
>>> r.outcome.chance, r.status
(0.6335, 'off_track')
>>> c = r.changes
>>> c.saving.to, c.start_value.to, c.amount.to, c.date.to
(2830.0, 73700.0, 136000.0, '2030-12')
>>> r.what_if.lower_returns.chance, r.what_if.crash.chance
(0.578, 0.012)
>>> w = r.what_if.crash_window
>>> w.from_, w.to, round(w.loss, 3)
('2007-12', '2008-11', -0.263)

```

The plan saves $2,500 a month towards $150,000 by June 2030, and buys a car for $25,000 on the
way; in 63% of the simulations it gets there, more than 15 points short of the 80% wanted. Any
one of saving $2,830 a month, starting with $73,700, aiming for $136,000 or waiting until
December 2030 would reach 80%. Returns a point a year lower take the chance to 58%; the worst
twelve months of the history, from December 2007, placed just before the date take it to 1%.

## Validation

`scripts/reference/planning.py` simulates four plans afresh from the definitions above, in
NumPy, with the historical model, which draws nothing at random: a target with a saving and a
car bought along the way (listed before the saving); a target with room to spare; payments after
five years of saving, along a glide path; and payments with room to spare. The chance, the percentiles of the value at the end and in every month, the
chance in each month, the share of the spending and of the payments made and the months the
money runs short, each change, the plan with lower returns and with the crash, and the mix's
risk agree to $10^{-9}$. Unit tests check the arithmetic on histories whose returns are known
(fees and inflation compounded, flows in money of the day, payments paid in part), the searches,
glide paths, and that a seed gives the same plan on any number of threads.

## Limits

Plans leave out taxes, life expectancy, spending rules that respond to markets, and several goals
at once; each plan tests one goal. The weights are rebalanced every month, without costs. The
bootstrap and historical models only replay months that happened, so no simulated month is worse
than the worst in the history; the normal model has fewer extreme months than markets have shown.

## References

- Politis, D. N. and Romano, J. P. (1994), "The Stationary Bootstrap", *Journal of the American
  Statistical Association* 89(428), 1303–1313.
