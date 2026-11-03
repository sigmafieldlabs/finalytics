"""The six reports on a portfolio (overview, holdings, performance, risk, attribution and
scenarios); the three reports on assets: the optimisation report of :meth:`Assets.optimise`,
the comparison report of :meth:`Assets.compare` and the backtest report of
:meth:`Assets.backtest`; and the rebalance report of :meth:`Portfolio.rebalance`.

Each report is computed in one call, by :meth:`Portfolio.report` (all six) or by a method such
as :meth:`Portfolio.performance_report`, and carries everything a screen draws and exports:
series over time, breakdowns, grids and distributions, each labelled with its units, and a
provenance with the engine version, the dates, the options and a SHA-256 digest of the inputs.
:func:`tables` turns any report into labelled tables for export::

    import finalytics as fl

    portfolio = fl.datasets.portfolio()
    report = portfolio.performance_report(benchmark="BENCH", group_by=["asset_class"])
    report.periods[0].cumulative
    for table in fl.reports.tables(report):
        print(table.title, len(table.rows))

Options are the fields of :class:`ReportOptions`, given as keywords or as a ``ReportOptions``.
"""

from __future__ import annotations

import dataclasses
import datetime
from collections.abc import Mapping
from typing import Any

from finalytics import _arrays, _finalytics, _types
from finalytics._convert import dumps, loads
from finalytics._errors import call
from finalytics._types import (
    AttributionInputs,
    AttributionReport,
    BacktestReport,
    BenchmarkPeriod,
    BenchmarkSegment,
    ComparisonReport,
    HoldingsReport,
    NamedScenario,
    OptimisationReport,
    OverviewReport,
    PerformanceReport,
    PortfolioReport,
    Provenance,
    RebalanceReport,
    ReportOptions,
    ReportTable,
    RiskReport,
    ScenariosReport,
    Source,
)

__all__ = [
    "AttributionInputs",
    "AttributionReport",
    "BacktestReport",
    "BenchmarkPeriod",
    "BenchmarkSegment",
    "ComparisonReport",
    "HoldingsReport",
    "NamedScenario",
    "OptimisationReport",
    "OverviewReport",
    "PerformanceReport",
    "PortfolioReport",
    "Provenance",
    "RebalanceReport",
    "ReportOptions",
    "ReportTable",
    "RiskReport",
    "ScenariosReport",
    "Source",
    "source",
    "tables",
]

_KINDS = {
    HoldingsReport: "holdings",
    OverviewReport: "overview",
    PerformanceReport: "performance",
    RiskReport: "risk",
    AttributionReport: "attribution",
    ScenariosReport: "scenarios",
    OptimisationReport: "optimisation",
    ComparisonReport: "comparison",
    BacktestReport: "backtest",
    RebalanceReport: "rebalance",
}


def _options(options: ReportOptions | None, keywords: dict[str, Any]) -> ReportOptions:
    for name in ("as_of", "start"):
        if keywords.get(name) is not None:
            keywords[name] = datetime.date.fromisoformat(_arrays.date(keywords[name]))
    for name in ("expected_returns", "benchmark_weights"):
        if isinstance(keywords.get(name), Mapping):
            keywords[name] = {k: float(v) for k, v in keywords[name].items()}
    base = options or ReportOptions()
    return dataclasses.replace(base, **keywords) if keywords else base


def _report(portfolio, kind: str, cls, options: ReportOptions | None, keywords: dict[str, Any]):
    text = call(portfolio._native.report, kind, dumps(_options(options, keywords)))
    return loads(cls, text)


def tables(report) -> list[ReportTable]:
    """Any report as labelled tables, for export to CSV, spreadsheets or PDF. A
    :class:`PortfolioReport` gives the tables of each of its reports in turn."""
    if isinstance(report, PortfolioReport):
        parts = [report.overview, report.holdings, report.performance, report.risk,
                 report.attribution, report.scenarios]
        return [table for part in parts if part is not None for table in tables(part)]
    if type(report) not in _KINDS:
        raise TypeError("tables() takes a report")
    return loads(list[ReportTable], call(_finalytics.report_tables, dumps(report)))


def source(portfolio) -> Source:
    """The digest and size of the inputs that the portfolio's reports are computed from."""
    return loads(Source, portfolio._native.source())
