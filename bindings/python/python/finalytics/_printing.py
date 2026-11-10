"""The reports and results as text and as an HTML page.

The report and result classes in :mod:`finalytics.types` derive from these, so that
``print(report)`` shows a report as text, 80 columns wide, and a report can be saved as one HTML
page or shown, inline in a notebook or in a browser, or by a handler set with
:func:`set_show_handler`. The results of :meth:`Assets.optimise`, :meth:`Assets.compare` and
:meth:`Assets.backtest` print and show their report, and their raw results print as text too.
"""

from __future__ import annotations

import html
import os
import pathlib
import sys
import tempfile
import webbrowser
from collections.abc import Callable
from typing import Literal

Theme = Literal["auto", "light", "dark"]
"""The colours of a report's page: ``"auto"`` (the default) follows the reader's light or dark
setting; ``"light"`` and ``"dark"`` keep to one."""


_show_handler: Callable[[str], object] | None = None


def set_show_handler(handler: Callable[[str], object] | None) -> None:
    """Sets what ``show()`` does with a report's page: from then on, ``show()`` calls `handler`
    with the page's HTML instead of showing it in a notebook or a browser, and returns ``None``.
    ``None`` restores the default.

    For programs that show pages themselves: a web page running Python in Pyodide, which has no
    browser to open, or an app that puts the page in a view of its own::

        import finalytics as fl

        pages = []
        fl.set_show_handler(pages.append)
        fl.Assets(fl.datasets.asset_class_prices()).optimise(method="minimum_variance").show()
        assert pages[0].startswith("<!doctype html>")
        fl.set_show_handler(None)
    """
    global _show_handler
    if handler is not None and not callable(handler):
        from finalytics._errors import invalid

        raise invalid(("handler", "must be a function that takes the page's HTML, or None"))
    _show_handler = handler


def _theme(theme: str) -> str:
    if theme not in ("auto", "light", "dark"):
        from finalytics._errors import invalid

        raise invalid(("theme", f"must be 'auto', 'light' or 'dark', not {theme!r}"))
    return theme


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


class ReportPage(PrintedReport):
    """A report that prints as text and lays out as one HTML page: the six reports on a
    portfolio together, the optimisation report and the backtest report."""

    __slots__ = ()

    def to_html(self, name: str | None = None, *, theme: Theme = "auto") -> str:
        """The report as one self-contained HTML page, with `name`, if given, as its title:
        inline styles and SVG charts with hover details, no scripts or external resources, in
        `theme` (the reader's light or dark setting by default, or ``"light"`` or ``"dark"``),
        and laid out for printing."""
        from finalytics import _finalytics
        from finalytics._convert import dumps
        from finalytics._errors import call

        return call(_finalytics.report_html, dumps(self), name, _theme(theme))

    def save_html(
        self, path: str | os.PathLike[str], name: str | None = None, *, theme: Theme = "auto"
    ) -> pathlib.Path:
        """Writes :meth:`to_html` to `path`, and returns the path."""
        target = pathlib.Path(path)
        target.write_text(self.to_html(name, theme=theme), encoding="utf-8")
        return target

    def show(self, *, theme: Theme = "auto") -> pathlib.Path | None:
        """Shows the report in `theme`: with the handler set by :func:`set_show_handler`, if
        there is one; else inline in a Jupyter notebook; and otherwise in the default browser,
        or with the command in the ``BROWSER`` environment variable if it is set, returning the
        path of the page it opens, as Rust and JavaScript do."""
        if _show_handler is not None:
            _show_handler(self.to_html(theme=theme))
            return None
        if _notebook():
            from IPython.display import display

            # As raw HTML, which IPython shows as it is, without its advice on frames.
            display({"text/html": self._frame(theme)}, raw=True)
            return None
        if sys.platform == "emscripten":
            raise RuntimeError(
                "cannot open a browser here; use to_html() for the page, or"
                " finalytics.set_show_handler() to say what show() does with it"
            )
        page = self.to_html(theme=theme)
        with tempfile.NamedTemporaryFile(
            "w", prefix="finalytics-report-", suffix=".html", delete=False, encoding="utf-8"
        ) as file:
            file.write(page)
        path = pathlib.Path(file.name)
        if not webbrowser.open(path.as_uri()):
            raise RuntimeError(f"cannot open a browser; the report is at {path}")
        return path

    def _repr_html_(self) -> str:
        return self._frame("auto")

    def _frame(self, theme: Theme) -> str:
        # The page keeps its own styles inside a frame, apart from the notebook's, and the frame
        # grows to the report's height where the notebook allows it.
        page = html.escape(self.to_html(theme=theme), quote=True)
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


class PrintedResults:
    """Raw results that print as text: the weights, or each strategy's periods, growth, trades
    and refits."""

    __slots__ = ()

    def to_text(self) -> str:
        """The results as text, as ``print(results)`` shows them."""
        from finalytics import _finalytics
        from finalytics._convert import dumps
        from finalytics._errors import call

        return call(_finalytics.results_text, dumps(self))

    def __str__(self) -> str:
        return self.to_text()


class Result:
    """What :meth:`Assets.optimise`, :meth:`Assets.compare` and :meth:`Assets.backtest` return:
    the report in ``report`` and the raw results in ``results``. ``print(result)`` prints the
    report and ``result.show()`` shows it."""

    __slots__ = ()

    def to_text(self, name: str | None = None) -> str:
        """The report as text for printing, 80 columns wide."""
        return self.report.to_text(name)  # type: ignore[attr-defined]

    def to_html(self, name: str | None = None, *, theme: Theme = "auto") -> str:
        """The report as one self-contained HTML page, in `theme`."""
        return self.report.to_html(name, theme=theme)  # type: ignore[attr-defined]

    def save_html(
        self, path: str | os.PathLike[str], name: str | None = None, *, theme: Theme = "auto"
    ) -> pathlib.Path:
        """Writes the report's page to `path`, and returns the path."""
        return self.report.save_html(path, name, theme=theme)  # type: ignore[attr-defined]

    def show(self, *, theme: Theme = "auto") -> pathlib.Path | None:
        """Shows the report in `theme`, as :meth:`ReportPage.show` does: with the handler set by
        :func:`set_show_handler`, if there is one; else inline in a Jupyter notebook; and
        otherwise in a browser, returning the path of the page it opens."""
        return self.report.show(theme=theme)  # type: ignore[attr-defined]

    def _repr_html_(self) -> str:
        return self.report._repr_html_()  # type: ignore[attr-defined]

    def __str__(self) -> str:
        return self.to_text()
