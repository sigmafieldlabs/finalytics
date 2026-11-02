"""The reports as text and as an HTML page.

The report classes in :mod:`finalytics.types` derive from these, so that ``print(report)`` shows
a report as text, 80 columns wide, and the full report can be saved as one HTML page.
"""

from __future__ import annotations

import os
import pathlib


class PrintedReport:
    """A report that prints as text."""

    __slots__ = ()

    def to_text(self, name: str | None = None) -> str:
        """The report as text for printing, 80 columns wide, with the portfolio's `name`, if
        given, under its title. ``print(report)`` shows the same text without a name."""
        from finalytics import _finalytics
        from finalytics._convert import dumps
        from finalytics._errors import call

        return call(_finalytics.report_text, dumps(self), name)

    def __str__(self) -> str:
        return self.to_text()


class PrintedPortfolioReport(PrintedReport):
    """The six reports, which print as text and save as one HTML page."""

    __slots__ = ()

    def to_html(self, name: str | None = None) -> str:
        """The six reports as one self-contained HTML page, with the portfolio's `name`, if
        given, as its title: inline styles and SVG charts with hover details, no scripts or
        external resources, in the reader's light or dark theme, and laid out for printing."""
        from finalytics import _finalytics
        from finalytics._convert import dumps
        from finalytics._errors import call

        return call(_finalytics.report_html, dumps(self), name)

    def save_html(self, path: str | os.PathLike[str], name: str | None = None) -> pathlib.Path:
        """Writes :meth:`to_html` to `path`, and returns the path."""
        target = pathlib.Path(path)
        target.write_text(self.to_html(name), encoding="utf-8")
        return target
