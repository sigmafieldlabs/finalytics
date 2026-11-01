import dataclasses
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
    objective = fl._convert.from_json(types.Objective, {"objective": "maximum_sharpe", "risk_free_rate": 0.02})
    assert objective == types.MaximumSharpe(risk_free_rate=0.02)
    assert fl._convert.to_json(objective) == {"objective": "maximum_sharpe", "risk_free_rate": 0.02}
    key = fl._convert.from_json(types.HoldingKey, {"holding": "cash", "id": "EUR"})
    assert key == fl.cash("EUR")
    exchange = fl._convert.from_json(
        types.Allocation, {"allocation": "exchange", "from": "A", "to": "B", "quantity": "2"}
    )
    assert exchange.from_ == "A" and fl._convert.to_json(exchange)["from"] == "A"


def test_the_public_module_lists_every_type():
    assert set(types.__all__) >= {"Statistics", "Solution", "Transaction", "Objective", "HoldingKey"}
    assert typing.get_type_hints(types.Solution)["weights"] == typing.Optional[list[float]]
