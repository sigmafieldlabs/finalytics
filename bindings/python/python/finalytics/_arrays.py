"""Moving dates and numbers to and from the engine as NumPy buffers."""

from __future__ import annotations

import datetime
from typing import Any

import numpy as np
import numpy.typing as npt

DateLike = datetime.date | str | np.datetime64
"""A date: a :class:`datetime.date`, an ISO 8601 string such as ``"2026-11-02"``, or a
:class:`numpy.datetime64`."""

FloatArray = npt.NDArray[np.float64]
DateArray = npt.NDArray[np.datetime64]


def date(value: DateLike | Any) -> str:
    """A date as ISO 8601 text."""
    if isinstance(value, datetime.datetime):
        return value.date().isoformat()
    if isinstance(value, datetime.date):
        return value.isoformat()
    if isinstance(value, np.datetime64):
        return str(value.astype("datetime64[D]"))
    if hasattr(value, "date") and callable(value.date):
        return value.date().isoformat()
    if isinstance(value, str):
        return datetime.date.fromisoformat(value[:10]).isoformat()
    raise TypeError(f"expected a date, found {type(value).__name__}")


def optional_date(value: DateLike | None) -> str | None:
    return None if value is None else date(value)


def dates_to_buffer(values: Any) -> bytes:
    days = np.asarray(values).astype("datetime64[D]").astype(np.int64)
    return days.astype("<i4").tobytes()


def dates_from_buffer(buffer: bytes) -> DateArray:
    return np.frombuffer(buffer, dtype="<i4").astype("datetime64[D]")


def floats_to_buffer(values: Any) -> bytes:
    return np.ascontiguousarray(values, dtype="<f8").tobytes()


def floats_from_buffer(buffer: bytes) -> FloatArray:
    return np.frombuffer(buffer, dtype="<f8").astype(np.float64)


def to_date(value: np.datetime64) -> datetime.date:
    return datetime.date.fromisoformat(str(value.astype("datetime64[D]")))
