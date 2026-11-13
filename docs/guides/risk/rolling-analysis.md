---
description: Compute rolling return, benchmark-relative and tail-risk statistics, align each window to its end date and choose a window size.
---

# Analyse rolling windows

Rolling measures repeat the same calculation over consecutive return windows. They reveal time variation that one full-history estimate hides, but each window contains fewer observations and therefore more estimation noise.

## Roll return and risk statistics

`rolling_statistics` returns one result for every complete window, in chronological order. The first result starts at the first return; each following window moves forward one observation.

[statistics.py](../../examples/rolling-analysis/statistics.py)

Options have the same meaning as in `statistics`: set periods per year and annual risk-free or target rates for the underlying return frequency.

## Roll benchmark-relative statistics

`rolling_relative_statistics` applies the same windows to paired portfolio and benchmark returns. It reports active return, tracking error, beta, correlation and the other relative measures for each window.

[relative.py](../../examples/rolling-analysis/relative.py)

Short windows can make beta and correlation unstable. Conditional betas and captures may be absent when a window lacks enough rising or falling benchmark periods.

## Roll tail risk

`rolling_tail_risk` supports the historical, parametric and Cornish–Fisher methods. A rolling historical tail estimate can move in steps as a severe observation enters or leaves the window.

[tail.py](../../examples/rolling-analysis/tail.py)

Choose the confidence and method with the window length in mind: a remote historical tail needs substantially more observations than a central risk estimate.

## Align windows with dates

The rolling functions return statistics, not dates. Window zero ends at the return at index `window - 1`; every later result ends one return later. Attach dates in your own data pipeline as shown here.

[dates.py](../../examples/rolling-analysis/dates.py)

This convention also applies to relative and tail-risk results. Missing or reordered input returns will produce misleading date labels, so validate the paired series first.

## Compare window sizes

A shorter window responds sooner to a new regime; a longer window is usually less noisy but slower to adapt. Changing the window also changes the number of complete results.

[window-size.py](../../examples/rolling-analysis/window-size.py)

The functions require a window of at least two returns and no longer than the input series. See [Return statistics and drawdowns](../../methods/statistics.md) for the underlying definitions and [Tail risk](tail-risk.md) for the loss conventions.
