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
        report = getattr(books, f"{name}_report")(options)
        assert report.provenance.report == name
        assert report.provenance.source == fl.reports.source(books)
        rectangular(fl.reports.tables(report))


def test_one_call_gives_every_report(books):
    options = fl.reports.ReportOptions(group_by=["asset_class"], benchmark="SPY", rolling_window=20)
    report = books.report(options)
    assert report.risk == books.risk_report(options)
    assert report.attribution is None and report.scenarios is None
    rectangular(fl.reports.tables(report))


def test_performance_matches_the_reference(books):
    report = books.performance_report(benchmark="SPY", as_of="2026-10-30")
    september, october = report.monthly.values[0][8:10]
    assert september == pytest.approx(REFERENCE["months"]["2026-09"], rel=1e-12)
    assert october == pytest.approx(REFERENCE["months"]["2026-10"], rel=1e-12)
    itd = next(p for p in report.periods if p.label == "ITD")
    assert itd.cumulative == pytest.approx(REFERENCE["periods"]["ITD"]["cumulative"], rel=1e-12)
    assert itd.benchmark == pytest.approx(REFERENCE["periods"]["ITD"]["benchmark"], rel=1e-12)


def test_holdings_and_concentration(books):
    report = books.holdings_report(group_by=["region"])
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
    report = books.attribution_report(attribution=inputs)
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
    report = books.scenarios_report(scenarios=[equities])
    assert report.summary[0].name == "Equities -20%"
    assert report.summary[0].change < 0
    assert report.scenarios[0].result.risk_after is not None
    rectangular(fl.reports.tables(report))


def test_options_are_checked(books):
    with pytest.raises(fl.ValidationError) as caught:
        books.risk_report(confidences=[1.5], benchmark="NOPE")
    paths = [i.path for i in caught.value.issues]
    assert paths == ["options.confidences[0]", "options.benchmark"]
    with pytest.raises(TypeError):
        fl.reports.tables(books)


def sheet_name_problems(tables):
    """The titles that Excel would refuse as sheet names, with why."""
    found, seen = [], set()
    for t in tables:
        title = t.title
        if not title.strip() or len(title) > 31 or any(c in title for c in "[]:*?/\\"):
            found.append(title)
        elif title.startswith("'") or title.endswith("'") or title.lower() in seen:
            found.append(title)
        seen.add(title.lower())
    return found


def test_table_titles_are_excel_sheet_names():
    portfolio = fl.datasets.portfolio()
    report = portfolio.report(fl.datasets.report_options())
    assert sheet_name_problems(fl.reports.tables(report)) == []
    assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    optimisation = assets.optimise(method="minimum_variance", max_weight=0.4)
    assert sheet_name_problems(fl.reports.tables(optimisation.report)) == []


def test_a_benchmark_in_another_currency_is_converted_and_a_blend_gives_the_segments():
    files = {name: fl.datasets.file(f"portfolio/{name}.csv") for name in ("transactions", "instruments", "accounts", "prices", "fx")}
    eur = fl.Portfolio.from_csv(**files, base_currency="EUR")
    report = eur.overview_report(benchmark="BENCH")
    assert report.provenance.benchmark == "BENCH, in USD, converted to EUR at the market data's rates"
    portfolio = fl.datasets.portfolio()
    blended = portfolio.report(benchmark={"VTI": 0.55, "BND": 0.35, "GLD": 0.1}, attribution="asset_class")
    assert blended.overview.provenance.benchmark == "a blend of 55% VTI, 35% BND and 10% GLD, rebalanced monthly"
    assert len(blended.attribution.periods) == 36
    # The portfolio's cash is a segment too, which the benchmark does not hold.
    first = {s.name: s.benchmark_weight for s in blended.attribution.periods[0].segments if s.benchmark_weight}
    assert first == pytest.approx({"Equity": 0.55, "Fixed income": 0.35, "Real assets": 0.1})
