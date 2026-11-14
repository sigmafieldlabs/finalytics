---
description: Review release notes, understand the bundled datasets and move existing code from Finalytics 0.9.
numbers: [0.9]
---

# Review releases, datasets and migration

This release's reference material includes its changes, the provenance and permitted role of its examples' data, and a concrete migration path for existing Finalytics code.

## Read the changelog

The changelog records additions, changes, fixes and removals by version, including release candidates on the way to the first stable release.

[Open the full changelog](../../CHANGELOG.md)

Use it to check when an interface appeared, whether a calculation or report changed, and whether an upgrade needs code changes. The `Unreleased` section describes work not yet in a published package.

## Understand the sample datasets

| Dataset | Contents | Data status |
| --- | --- | --- |
| Asset classes | Six total-return indices at monthly frequency | Synthetic, drawn with a fixed seed |
| Household portfolio | Two accounts, five funds, cash, transactions, prices, FX and report options | Real fund prices and distributions; illustrative transactions |
| Screening universe | Fictional companies across eleven sectors, with prices, volume, dividends, fields and an index | Synthetic, drawn identically on every platform |

The household's real prices, distributions and EUR/USD rates come from Yahoo Finance. The bundled examples use them for demonstration and testing, not as a current investment recommendation. The asset-class indices and companies are invented; their simulated crises are illustrative rather than historical reconstructions.

`datasets.files()` lists the text files exposed by each package. `datasets.file(name)` returns one for inspecting layouts or supplying another front door. The Python pandas variants and JavaScript table forms contain the same data.

See [Bring your own data](../data/bring-your-own-data.md) to replace the samples with data you are entitled to use. Finalytics itself downloads no market data.

## Migrate from Finalytics 0.9

Version 1.0 is a redesign rather than a drop-in update. You supply prices and other data, `Assets` handles return-based analysis and optimisation, and `Portfolio` keeps transaction-based books. Results expose both a report and the numbers behind it.

[Open the migration guide](../../migrating-from-0.9.md)

The guide includes a replacement table for old names and a worked optimisation with asset and group constraints. It also explains removed downloading and reporting paths, package and licence changes, and how to pin the older release while migrating.

> [!WARNING]
> Do not change an existing application's version and assume equivalent output. Replace data fetching first, migrate records and optimisation options, then compare results under the conventions documented in Methods and validation.
