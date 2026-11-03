"""The reports as text and as an HTML page.

The report classes in :mod:`finalytics.types` derive from these, so that ``print(report)`` shows
a report as text, 80 columns wide, and the full report can be saved as one HTML page or shown,
inline in a notebook or in a browser.
"""

from __future__ import annotations

import html
import os
import pathlib
import sys
import tempfile
import webbrowser


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

    def show(self) -> None:
        """Shows the report: inline in a Jupyter notebook, and otherwise in the default
        browser, or with the command in the ``BROWSER`` environment variable if it is set."""
        if _notebook():
            from IPython.display import HTML, display

            display(HTML(self._repr_html_()))
            return
        if sys.platform == "emscripten":
            raise RuntimeError("cannot open a browser here; use to_html() for the page")
        with tempfile.NamedTemporaryFile(
            "w", prefix="finalytics-report-", suffix=".html", delete=False, encoding="utf-8"
        ) as file:
            file.write(self.to_html())
        path = pathlib.Path(file.name)
        if not webbrowser.open(path.as_uri()):
            raise RuntimeError(f"cannot open a browser; the report is at {path}")

    def _repr_html_(self) -> str:
        # The page keeps its own styles inside a frame, apart from the notebook's, and the frame
        # grows to the report's height where the notebook allows it.
        page = html.escape(self.to_html(), quote=True)
        grow = "this.style.height = this.contentDocument.documentElement.scrollHeight + 'px'"
        return (
            f'<iframe srcdoc="{page}" title="Report" onload="{grow}" '
            'style="width: 100%; height: 900px; border: 0"></iframe>'
        )


def _notebook() -> bool:
    """Whether the code runs in a notebook, rather than a terminal or a script."""
    ipython = sys.modules.get("IPython")
    if ipython is None:
        return False
    shell = ipython.get_ipython()
    return shell is not None and type(shell).__name__ != "TerminalInteractiveShell"
