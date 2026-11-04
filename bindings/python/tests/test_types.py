import dataclasses
import datetime
import json
import typing

import finalytics as fl
from finalytics import _types, types

from conftest import ROOT

SCHEMA = json.loads((ROOT / "schema/finalytics.schema.json").read_text())["$defs"]


def test_every_struct_matches_its_schema():
    for name, schema in SCHEMA.items():
        cls = getattr(_types, name, None)
        if cls is None or not dataclasses.is_dataclass(cls) or "oneOf" in schema:
            continue
        fields = [f.name.rstrip("_") for f in dataclasses.fields(cls)]
        assert fields == list(schema["properties"]), name


def test_unions_dispatch_on_their_tags():
    objective = fl._convert.from_json(types.Objective, {"type": "maximum_sharpe", "risk_free_rate": 0.02})
    assert objective == types.MaximumSharpe(risk_free_rate=0.02)
    assert fl._convert.to_json(objective) == {"type": "maximum_sharpe", "risk_free_rate": 0.02}
    assert fl._convert.from_json(types.HoldingKey, "EUR cash") == fl.cash("EUR") == "EUR cash"
    exchange = fl._convert.from_json(types.Allocation, {"type": "exchange", "from": "A", "to": "B", "quantity": "2"})
    assert exchange.from_ == "A" and fl._convert.to_json(exchange)["from"] == "A"


def test_the_public_module_lists_every_type():
    assert set(types.__all__) >= {"Statistics", "Solution", "Transaction", "Objective", "HoldingKey"}
    assert typing.get_type_hints(types.Solution)["weights"] == typing.Optional[list[float]]


def test_every_public_module_exports_what_it_lists():
    for module in (fl, fl.opt, fl.performance, fl.reports, fl.risk, fl.types):
        missing = [name for name in module.__all__ if not hasattr(module, name)]
        assert not missing, (module.__name__, missing)
    assert "performance" in fl.__all__


def test_a_holdout_takes_its_date_as_text():
    holdout = fl.Holdout(start="2016-11-01")
    assert holdout.start == datetime.date(2016, 11, 1)
    assert holdout == fl.Holdout(start=datetime.date(2016, 11, 1))
    assert fl._convert.from_json(types.Validation, fl._convert.to_json(holdout)) == holdout
