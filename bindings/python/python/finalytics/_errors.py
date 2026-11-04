"""Errors raised by Finalytics."""

from __future__ import annotations

import json
from collections.abc import Callable
from typing import TypeVar

from finalytics import _finalytics
from finalytics._types import Issue

T = TypeVar("T")


class FinalyticsError(Exception):
    """An error from the Finalytics engine."""


class ValidationError(FinalyticsError, ValueError):
    """Input failed validation.

    :attr:`issues` lists every problem found, each with a path to where it is (for example
    ``transactions[14].quantity``) and a plain description, ready to show to a user.
    """

    def __init__(self, message: str, issues: list[Issue]):
        super().__init__(message)
        self.issues = issues
        """Every problem found."""


def invalid(*problems: tuple[str, str]) -> ValidationError:
    """A :class:`ValidationError` for problems found before calling the engine, each a path
    (the keyword, such as ``returns``) and a message, worded as the engine words its own."""
    issues = [Issue(path=path, message=message, severity="error") for path, message in problems]
    lines = "".join(f"\n  error: {i.path}: {i.message}" for i in issues)
    return ValidationError(f"validation found {len(issues)} error(s){lines}", issues)


def call(function: Callable[..., T], *args) -> T:
    """Calls into the engine, raising this package's errors."""
    try:
        return function(*args)
    except _finalytics.EngineError as error:
        message, issues = error.args if len(error.args) == 2 else (str(error), None)
        if issues is None:
            raise FinalyticsError(message) from None
        found = [
            Issue(path=i["path"], message=i["message"], severity=i["severity"]) for i in json.loads(issues)
        ]
        raise ValidationError(message, found) from None
