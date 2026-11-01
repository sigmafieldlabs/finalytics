import pytest

pd = pytest.importorskip("pandas")

import finalytics.pandas as flpd  # noqa: E402


def test_series_and_frames(books):
    perf = books.performance()
    returns = flpd.returns(perf)
    assert len(returns) == 43 and str(returns.index[0].date()) == "2026-09-02"
    contributions = flpd.frame(perf.contributions())
    assert set(contributions["holding"]) >= {"SPY", "SAP", "BOND", "EUR cash"}
    assert contributions["contribution"].sum() == pytest.approx(perf.time_weighted(), abs=1e-12)
