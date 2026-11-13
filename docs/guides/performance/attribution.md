---
description: Explain active return with Brinson–Fachler effects, linked periods and Karnosky–Singer currency attribution.
learn: portfolio/2.3
---

# Attribution

Attribution explains return relative to a benchmark. Brinson–Fachler separates segment allocation, security selection and their interaction; Karnosky–Singer adds currency allocation.

## One-period Brinson–Fachler

Each segment needs portfolio and benchmark weights and returns. Weights sum to one on each side, with cash represented as a segment where relevant.

[single-period.py](../../examples/attribution/single-period.py)

## Link several periods

Within one period the effects add to arithmetic excess return. Cariño linking scales period effects so they add to the excess of compounded returns over the full interval.

[linked.py](../../examples/attribution/linked.py)

## Compare Menchero linking

Menchero distributes the linking adjustment differently while preserving the same total. Use one convention consistently when comparing reports.

[menchero.py](../../examples/attribution/menchero.py)

## Currency attribution

Karnosky–Singer measures market effects on returns above each currency's cash return and adds currency allocation. Without explicit exposures, segment weights are unhedged currency exposures and the residual contains cross terms.

[currency.py](../../examples/attribution/currency.py)

## Represent hedging

Explicit portfolio exposures change currency allocation and remove the portfolio's unhedged cross-term residual. Benchmark exposures can be supplied independently.

[hedged.py](../../examples/attribution/hedged.py)

> [!WARNING]
> Linking requires every period's portfolio and benchmark return to be above -100%. A single period with a larger loss can be analysed alone but cannot be linked with these methods.
