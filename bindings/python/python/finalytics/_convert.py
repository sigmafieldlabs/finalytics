"""Conversion between the dataclasses in :mod:`finalytics.types` and the engine's JSON."""

from __future__ import annotations

import dataclasses
import datetime
import decimal
import functools
import json
import keyword
import types
import typing
from typing import Any

from finalytics import _types

_UNIONS = {getattr(_types, name): (tag, classes) for name, (tag, classes) in _types.UNIONS.items()}
_TAGS = {cls: (tag, value) for tag, classes in _UNIONS.values() for value, cls in classes.items()}
_FLATTENED = {
    getattr(_types, name): (field, getattr(_types, union))
    for name, (field, union) in _types.FLATTENED.items()
}


def _attribute(key: str) -> str:
    return key + "_" if keyword.iskeyword(key) else key


def _key(attribute: str) -> str:
    stem = attribute[:-1]
    return stem if attribute.endswith("_") and keyword.iskeyword(stem) else attribute


@functools.cache
def _hints(cls: type) -> dict[str, Any]:
    return typing.get_type_hints(cls)


def _lookup(table: dict, cls: type):
    for base in cls.__mro__:
        if base in table:
            return table[base]
    return None


def from_json(tp: Any, value: Any) -> Any:
    """Builds a value of type `tp` from decoded JSON."""
    if value is None:
        return None
    if tp in _UNIONS:
        tag, classes = _UNIONS[tp]
        return _build(classes[value[tag]], value)
    origin = typing.get_origin(tp)
    if origin in (typing.Union, types.UnionType):
        options = [a for a in typing.get_args(tp) if a is not type(None)]
        if len(options) == 1:
            return from_json(options[0], value)
        if all(o in (float, int, bool, str) for o in options):
            return value
        variants = [a for a in options if a in _TAGS]
        if variants and isinstance(value, dict):
            for cls in variants:
                tag, name = _TAGS[cls]
                if value.get(tag) == name:
                    return _build(cls, value)
        raise TypeError(f"cannot read {value!r} as {tp}")
    if origin is list:
        (item,) = typing.get_args(tp)
        return [from_json(item, v) for v in value]
    if origin is tuple:
        return tuple(from_json(t, v) for t, v in zip(typing.get_args(tp), value))
    if origin is dict:
        key, item = typing.get_args(tp)
        return {(int(k) if key is int else k): from_json(item, v) for k, v in value.items()}
    if origin is typing.Literal:
        return value
    if tp is datetime.date:
        return datetime.date.fromisoformat(value)
    if tp is decimal.Decimal:
        return decimal.Decimal(str(value))
    if tp is float:
        return float(value)
    if dataclasses.is_dataclass(tp):
        return _build(tp, value)
    return value


def _build(cls: type, value: dict) -> Any:
    hints = _hints(cls)
    names = {f.name for f in dataclasses.fields(cls)}
    flat = _lookup(_FLATTENED, cls)
    kwargs = {}
    rest = {}
    for key, v in value.items():
        name = _attribute(key)
        if name in names and (flat is None or name != flat[0]):
            kwargs[name] = from_json(hints[name], v)
        else:
            rest[key] = v
    if flat is not None:
        field, union = flat
        kwargs[field] = from_json(union, rest)
    return cls(**kwargs)


def to_json(value: Any) -> Any:
    """Turns a value built from :mod:`finalytics.types` (or plain Python data) into JSON data."""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, (int, float)):
        return value
    if isinstance(value, decimal.Decimal):
        return str(value)
    if isinstance(value, datetime.datetime):
        return value.date().isoformat()
    if isinstance(value, datetime.date):
        return value.isoformat()
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        out: dict[str, Any] = {}
        tag = _lookup(_TAGS, type(value))
        if tag is not None:
            out[tag[0]] = tag[1]
        flat = _lookup(_FLATTENED, type(value))
        for f in dataclasses.fields(value):
            v = getattr(value, f.name)
            if v is None:
                continue
            if flat is not None and f.name == flat[0]:
                out.update(to_json(v))
            else:
                out[_key(f.name)] = to_json(v)
        return out
    if isinstance(value, dict):
        return {k if isinstance(k, str) else str(k): to_json(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_json(v) for v in value]
    if hasattr(value, "tolist"):
        return value.tolist()
    if hasattr(value, "item"):
        return value.item()
    raise TypeError(f"cannot write {type(value).__name__} as JSON")


def dumps(value: Any) -> str:
    """JSON text for `value`."""
    return json.dumps(to_json(value), allow_nan=False)


def loads(tp: Any, text: str) -> Any:
    """A value of type `tp` from JSON text."""
    return from_json(tp, json.loads(text))
