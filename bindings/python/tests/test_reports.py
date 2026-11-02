import json

import pytest

import finalytics as fl

from conftest import FIXTURES

REFERENCE = json.loads((FIXTURES / "finalytics/tests/fixtures/reports.json").read_text())


def rectangular(tables):
    assert tables
    for t in tables:
        assert all(len(row) == len(t.columns) for row in t.rows), t.name


def test_every_report_runs_and_exports(books):
    options = fl.reports.ReportOptions(group_by=["asset_class"], benchmark="SPY", rolling_window=20)
    for name in ("holdings", "overview", "performance", "risk"):
        report = getattr(fl.reports, name)(books, options)
        assert report.provenance.report == name
        assert report.provenance.source == fl.reports.source(books)
        rectangular(fl.reports.tables(report))


def test_performance_matches_the_reference(books):
    report = fl.reports.performance(books, benchmark="SPY", as_of="2026-10-30")
    september, october = report.monthly.values[0][8:10]
    assert september == pytest.approx(REFERENCE["months"]["2026-09"], rel=1e-12)
    assert october == pytest.approx(REFERENCE["months"]["2026-10"], rel=1e-12)
    itd = next(p for p in report.periods if p.label == "ITD")
    assert itd.cumulative == pytest.approx(REFERENCE["periods"]["ITD"]["cumulative"], rel=1e-12)
    assert itd.benchmark == pytest.approx(REFERENCE["periods"]["ITD"]["benchmark"], rel=1e-12)


def test_holdings_and_concentration(books):
    report = fl.reports.holdings(books, group_by=["region"])
    c = report.concentration
    assert c.herfindahl == pytest.approx(REFERENCE["concentration"]["herfindahl"], rel=1e-12)
    assert sum(h.value_base for h in report.holdings) == report.value
    assert [b.by for b in report.breakdowns][-1] == "region"


def test_attribution_and_scenarios(books):
    segments = [
        fl.reports.BenchmarkSegment(name="Equity", benchmark_weight=0.6, benchmark_return=-0.04),
        fl.reports.BenchmarkSegment(name="Fixed income", benchmark_weight=0.4, benchmark_return=0.01),
    ]
    import datetime

    inputs = fl.reports.AttributionInputs(
        scheme="asset_class",
        periods=[
            fl.reports.BenchmarkPeriod(
                start=datetime.date(2026, 9, 1), end=datetime.date(2026, 9, 30), segments=segments
            )
        ],
    )
    report = fl.reports.attribution(books, attribution=inputs)
    expected = REFERENCE["attribution"][0]["segments"]
    found = {s.name: s for s in report.periods[0].segments}
    for e in expected:
        assert found[e["name"]].portfolio_weight == pytest.approx(e["portfolio_weight"], rel=1e-12, abs=1e-15)
        assert found[e["name"]].portfolio_return == pytest.approx(e["portfolio_return"], rel=1e-12, abs=1e-15)
    rectangular(fl.reports.tables(report))

    equities = fl.reports.NamedScenario(
        name="Equities -20%",
        scenario=fl.types.ShockScenario(
            shocks=fl.risk.Shocks(groups=[fl.risk.GroupShock(scheme="asset_class", value="Equity", shock=-0.2)])
        ),
    )
    report = fl.reports.scenarios(books, scenarios=[equities])
    assert report.summary[0].name == "Equities -20%"
    assert report.summary[0].change < 0
    assert report.scenarios[0].result.risk_after is not None
    rectangular(fl.reports.tables(report))


def test_options_are_checked(books):
    with pytest.raises(fl.ValidationError) as caught:
        fl.reports.risk(books, confidences=[1.5], benchmark="NOPE")
    paths = [i.path for i in caught.value.issues]
    assert paths == ["options.confidences[0]", "options.benchmark"]
    with pytest.raises(TypeError):
        fl.reports.tables(books)
