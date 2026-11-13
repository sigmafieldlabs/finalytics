---
description: Understand Assets, Portfolio and Universe, and move between typed results, readable reports and precise validation errors.
---

# How Finalytics works

Finalytics has three front doors. `Assets` analyses prices or returns, `Portfolio` keeps the books from transactions, and `Universe` screens many securities. All three use data you supply; the library does not fetch market data.

## Choose a front door

Use `Assets` for statistics and optimisation, `Portfolio` for valuation and performance, and `Universe` for screening. The packaged datasets make each one easy to try.

[front-doors.py](../../examples/how-finalytics-works/front-doors.py)

## Results and reports

An operation returns structured `results` for your own calculations and a `report` for presentation. Print the operation for an 80-column text report, or call `show()` for its HTML page. In these guides, Run regenerates both where the example shows a report.

[results-and-reports.py](../../examples/how-finalytics-works/results-and-reports.py)

## Discover methods

Method names are data, not something to memorise. `methods()` lists the optimisation methods and their display names in every language.

[methods.py](../../examples/how-finalytics-works/methods.py)

## Read every validation issue

Invalid options raise `ValidationError`. Its `issues` include a precise path and a readable message, and independent problems are reported together. Rust returns the same report as an `Error::Validation` value.

[errors.py](../../examples/how-finalytics-works/errors.py)

> [!NOTE]
> Python and JavaScript options use snake_case, just like the underlying records. JavaScript method names use camelCase; Rust uses builder methods and enums.

## What happens next

Continue with **Your first portfolio** for books and reports, or **Your first optimisation** for weights and frontiers. **Bring your own data** shows the exact CSV layouts when you are ready to use your own records.
