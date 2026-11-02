import json

import numpy as np
import pandas as pd
import pytest

import finalytics as fl
from finalytics._convert import loads

from conftest import FIXTURES

ATTRIBUTION = json.loads((FIXTURES / "finalytics-perf/tests/fixtures/attribution.json").read_text())


@pytest.mark.parametrize("linking", ["carino", "menchero"])
def test_brinson_fachler_matches_the_reference(linking):
    case = ATTRIBUTION["brinson_fachler"]
    periods = loads(list[fl.types.AttributionPeriod], json.dumps(case["periods_input"]))
    found = fl.performance.brinson_fachler(periods, linking)
    assert found.portfolio_return == pytest.approx(case["portfolio_return"], rel=1e-14)
    expected = case[linking]
    coefficients = [p.linking_coefficient for p in found.periods]
    np.testing.assert_allclose(coefficients, expected["coefficients"], rtol=1e-13, atol=0)
    np.testing.assert_allclose([s.allocation for s in found.segments], expected["allocation"], rtol=0, atol=1e-15)
    np.testing.assert_allclose([s.selection for s in found.segments], expected["selection"], rtol=0, atol=1e-15)
    assert found.allocation + found.selection + found.interaction == pytest.approx(found.excess_return, abs=1e-15)


def test_periods_from_a_frame():
    frame = pd.DataFrame(
        {
            "portfolio_weight": [0.4, 0.3, 0.3],
            "portfolio_return": [0.20, -0.05, 0.06],
            "benchmark_weight": [0.4, 0.2, 0.4],
            "benchmark_return": [0.10, -0.04, 0.08],
        },
        index=["UK", "Japan", "US"],
    )
    period = fl.performance.attribution_period("2026-01-01", "2026-12-31", frame)
    found = fl.performance.brinson_fachler([period])
    assert [s.name for s in found.segments] == ["UK", "Japan", "US"]
    assert found.allocation == pytest.approx(-0.012, abs=1e-15)
    assert found.selection == pytest.approx(0.030, abs=1e-15)
    assert found.interaction == pytest.approx(0.001, abs=1e-15)


def test_karnosky_singer_matches_the_reference():
    case = ATTRIBUTION["karnosky_singer"]
    periods = loads(list[fl.types.CurrencyAttributionPeriod], json.dumps(case["periods_input"]))
    found = fl.performance.karnosky_singer(periods)
    np.testing.assert_allclose([c.allocation for c in found.currencies], case["carino"]["currency"], rtol=0, atol=1e-15)
    assert found.residual == pytest.approx(case["carino"]["residual"], abs=1e-15)
    total = found.allocation + found.selection + found.interaction + found.currency + found.residual
    assert total == pytest.approx(found.excess_return, abs=1e-15)
