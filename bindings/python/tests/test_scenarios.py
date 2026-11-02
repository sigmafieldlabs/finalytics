import json

import numpy as np
import pytest

import finalytics as fl
from finalytics._convert import loads

from conftest import FIXTURES

SCENARIOS = json.loads((FIXTURES / "finalytics/tests/fixtures/scenarios.json").read_text())


def label(key) -> str:
    return key.id if isinstance(key, fl.types.InstrumentHolding) else f"{key.id} cash"


def compare(result, expected):
    assert result.value_before == pytest.approx(expected["value_before"], rel=1e-12)
    assert result.value_after == pytest.approx(expected["value_after"], rel=1e-12)
    assert [label(h.holding) for h in result.holdings] == [h["label"] for h in expected["holdings"]]
    for name in ("local_return", "currency_return", "total_return", "value_after"):
        np.testing.assert_allclose(
            [getattr(h, name) for h in result.holdings],
            [h[name] for h in expected["holdings"]],
            rtol=1e-12,
            atol=1e-12,
        )


def test_shocks_match_the_reference(books):
    case = SCENARIOS["shocks"]
    shocks, options = case["shocks"], case["options"]
    covariance = loads(fl.risk.Matrix, json.dumps(options["risk"]["covariance"]))
    result = books.shock_scenario(
        instruments=dict(shocks["instruments"]),
        groups={(g["scheme"], g["value"]): g["shock"] for g in shocks["groups"]},
        currencies=dict(shocks["currencies"]),
        date=case["as_of"],
        group_by=options["group_by"],
        risk=covariance,
        confidence=options["risk"]["confidence"],
        stress=fl.risk.CorrelationStress(**options["risk"]["stress"]),
    )
    expected = case["expected"]
    compare(result, expected)
    by_region = {g.group: g.contribution for g in result.groups[1].groups}
    assert by_region == pytest.approx({g["group"]: g["contribution"] for g in expected["groups"]["region"]}, rel=1e-12)
    for found, key in ((result.risk_before, "risk_before"), (result.risk_after, "risk_after")):
        assert found.volatility == pytest.approx(expected[key]["volatility"], rel=1e-12)
        assert found.value_at_risk == pytest.approx(expected[key]["value_at_risk"], rel=1e-12)
    sap = next(h for h in result.holdings if label(h.holding) == "SAP")
    assert isinstance(sap.source, fl.types.InstrumentSource)
    spy = next(h for h in result.holdings if label(h.holding) == "SPY")
    assert spy.source == fl.types.GroupSource(scheme="asset_class", value="Equity")


@pytest.mark.parametrize("key", ["factors_one", "factors_two", "factors_alone"])
def test_factor_shocks_match_the_reference(books, key):
    case = SCENARIOS[key]
    factors = case["shocks"]["factors"]
    model = loads(fl.risk.FactorModel, json.dumps(factors["model"]))
    result = books.shock_scenario(
        factors=dict(factors["shocks"]),
        factor_model=model,
        propagate=factors["propagate"],
        date=case["as_of"],
    )
    compare(result, case["expected"])
    np.testing.assert_allclose([f for _, f in result.factor_returns], case["expected"]["factor_returns"], rtol=1e-12)


@pytest.mark.parametrize("key", ["historical", "historical_proxy"])
def test_historical_replays_match_the_reference(books, key):
    case = SCENARIOS[key]
    window = case["window"]
    result = books.historical_scenario(
        window["start"], window["end"], proxies=dict(window["proxies"]), date=case["as_of"]
    )
    compare(result, case["expected"])
    path = case["expected"]["path"]
    assert [d.isoformat() for d in result.path.dates] == path["dates"]
    np.testing.assert_allclose(result.path.values, path["values"], rtol=1e-12)
    assert result.path.worst_drawdown.trough.isoformat() == path["trough"]


def test_overlapping_group_shocks_are_an_error(books):
    with pytest.raises(fl.ValidationError) as caught:
        books.shock_scenario(groups={("asset_class", "Equity"): -0.2, ("region", "Europe"): -0.3})
    assert [i.path for i in caught.value.issues] == ["shocks.groups"]
    assert "SAP is in more than one shocked group" in caught.value.issues[0].message


def test_unshocked_instruments_are_listed(books):
    result = books.shock_scenario(instruments={"SPY": -0.1})
    assert result.not_shocked == ["BOND", "SAP"]


def test_stressed_covariance():
    covariance = fl.risk.matrix([[0.04, 0.0], [0.0, 0.09]], ["a", "b"])
    stressed = fl.risk.stressed_covariance(covariance, volatility_scale=2.0, correlation_shift=1.0)
    np.testing.assert_allclose(fl.risk.to_array(stressed), 4 * np.array([[0.04, 0.06], [0.06, 0.09]]), rtol=1e-15)
