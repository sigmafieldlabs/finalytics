import datetime

import numpy as np
import pytest

import finalytics as fl


def test_daily_returns_and_growth(books):
    perf = books.performance()
    assert len(perf.dates) == 44 and len(perf.returns) == 43
    assert perf.dates.dtype == np.dtype("datetime64[D]")
    assert perf.growth[0] == 1.0
    np.testing.assert_allclose(np.cumprod(1 + perf.returns), perf.growth[1:], rtol=1e-12)
    assert perf.time_weighted() == pytest.approx(perf.growth[-1] - 1, rel=1e-12)
    assert "Performance(2026-09-01 to 2026-10-30" in repr(perf)


def test_periods_and_money_weighted(books):
    perf = books.performance()
    mtd = perf.period("MTD")
    assert mtd.start.isoformat() == "2026-09-30" and mtd.annualised is None
    assert perf.period("1Y") is None
    periods = perf.periods(["MTD", "ITD"])
    assert periods["ITD"].cumulative == pytest.approx(perf.time_weighted(), rel=1e-12)
    assert perf.money_weighted() is not None
    assert perf.modified_dietz() == pytest.approx(perf.time_weighted(), abs=0.02)
    with pytest.raises(ValueError):
        fl.period("2W")
    assert fl.period(" ytd ") == fl.types.YearToDate()
    assert fl.period("since 2026-01-31") == fl.types.Since(start=datetime.date(2026, 1, 31))


def test_contributions_add_up(books, portfolio):
    perf = books.performance()
    contributions = perf.contributions()
    assert sum(c.contribution for c in contributions) == pytest.approx(perf.time_weighted(), abs=1e-12)
    classes = {i.id: i.classifications["asset_class"] for i in portfolio.instruments}

    def group(key):
        return "Cash" if key.endswith(" cash") else classes[key]

    groups = fl.group_contributions(contributions, group)
    assert [g.group for g in groups] == ["Cash", "Equity", "Fixed income"]
    assert sum(g.contribution for g in groups) == pytest.approx(perf.time_weighted(), abs=1e-12)


def test_excess_return_and_values_only(books):
    perf = books.performance()
    levels = perf.growth * 1.01
    excess = perf.excess_return(levels, "ITD", dates=perf.dates)
    assert excess.arithmetic == pytest.approx(0.0, abs=1e-12)

    simple = fl.Performance.from_values(["2026-01-02", "2026-01-05", "2026-01-06"], [100.0, 110.0, 99.0])
    np.testing.assert_allclose(simple.returns, [0.1, -0.1])
    with pytest.raises(fl.ValidationError):
        fl.Performance.from_values(["2026-01-02", "2026-01-05"], [100.0, 50.0], [0.0, -200.0])
