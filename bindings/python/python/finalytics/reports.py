"""Report data for each workstation screen: holdings, overview, performance, risk, attribution
and scenarios.

Each report is computed in one call from a portfolio's books, and carries everything its screen
draws and exports: series over time, breakdowns, grids, distributions, each labelled with its
units, and a provenance with the engine version, the dates, the options and a SHA-256 digest of
the inputs. :func:`tables` turns any report into labelled tables for export::

    import finalytics as fl

    books = portfolio.books(market)
    report = fl.reports.performance(books, benchmark="SPX", group_by=["asset_class"])
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
    BenchmarkPeriod,
    BenchmarkSegment,
    HoldingsReport,
    NamedScenario,
    OverviewReport,
    PerformanceReport,
    Provenance,
    ReportOptions,
    ReportTable,
    RiskReport,
    ScenariosReport,
    Source,
)

__all__ = [
    "AttributionInputs",
    "AttributionReport",
    "BenchmarkPeriod",
    "BenchmarkSegment",
    "HoldingsReport",
    "NamedScenario",
    "OverviewReport",
    "PerformanceReport",
    "Provenance",
    "ReportOptions",
    "ReportTable",
    "RiskReport",
    "ScenariosReport",
    "Source",
    "attribution",
    "holdings",
    "overview",
    "performance",
    "risk",
    "scenarios",
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
}


def _options(options: ReportOptions | None, keywords: dict[str, Any]) -> ReportOptions:
    for name in ("as_of", "start"):
        if keywords.get(name) is not None:
            keywords[name] = datetime.date.fromisoformat(_arrays.date(keywords[name]))
    if isinstance(keywords.get("expected_returns"), Mapping):
        keywords["expected_returns"] = [(k, float(v)) for k, v in keywords["expected_returns"].items()]
    if isinstance(keywords.get("benchmark_weights"), Mapping):
        from finalytics.portfolio import _holding_key

        keywords["benchmark_weights"] = [(_holding_key(k), float(v)) for k, v in keywords["benchmark_weights"].items()]
    base = options or ReportOptions()
    return dataclasses.replace(base, **keywords) if keywords else base


def _report(books, kind: str, cls, options: ReportOptions | None, keywords: dict[str, Any]):
    text = call(books._native.report, kind, dumps(_options(options, keywords)))
    return loads(cls, text)


def holdings(books, options: ReportOptions | None = None, **keywords: Any) -> HoldingsReport:
    """Positions, lots and cash by holding and by account; allocation by holding, currency,
    instrument kind, account and each scheme of ``group_by``; concentration; and realised gains
    and income to date."""
    return _report(books, "holdings", HoldingsReport, options, keywords)


def overview(books, options: ReportOptions | None = None, **keywords: Any) -> OverviewReport:
    """Headline figures, value and growth against the benchmark, allocation, risk by holding and
    by group, each holding's expected return and risk, and the efficient frontier with the
    current portfolio."""
    return _report(books, "overview", OverviewReport, options, keywords)


def performance(books, options: ReportOptions | None = None, **keywords: Any) -> PerformanceReport:
    """Value and growth over time, returns over the standard periods against the benchmark,
    monthly and calendar-year returns, contribution by holding and by group, flows and income
    by month, and the household by account."""
    return _report(books, "performance", PerformanceReport, options, keywords)


def risk(books, options: ReportOptions | None = None, **keywords: Any) -> RiskReport:
    """Statistics, drawdowns, value at risk and expected shortfall by every method, the return
    distribution, the holdings' covariance, correlation and risk decomposition, and rolling
    statistics."""
    return _report(books, "risk", RiskReport, options, keywords)


def attribution(books, options: ReportOptions | None = None, **keywords: Any) -> AttributionReport:
    """Brinson–Fachler attribution against the benchmark segments given as ``attribution``,
    with the portfolio's segments measured from its books."""
    return _report(books, "attribution", AttributionReport, options, keywords)


def scenarios(books, options: ReportOptions | None = None, **keywords: Any) -> ScenariosReport:
    """The ``scenarios`` given, run together on the current holdings with the holdings'
    covariance as the risk model, side by side."""
    return _report(books, "scenarios", ScenariosReport, options, keywords)


def tables(report) -> list[ReportTable]:
    """Any report as labelled tables, for export to CSV, spreadsheets or PDF."""
    if type(report) not in _KINDS:
        raise TypeError("tables() takes a report")
    return loads(list[ReportTable], call(_finalytics.report_tables, dumps(report)))


def source(books) -> Source:
    """The digest and size of the inputs that the books' reports are computed from."""
    return loads(Source, books._native.source())
