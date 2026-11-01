"""Finalytics: portfolio analytics for Python.

Finalytics keeps the books of a multi-currency portfolio from its transactions, measures
performance and risk, and optimises portfolios under real-world constraints. The engine is
written in Rust; this package is a thin, typed layer over it.
"""

from importlib.metadata import version as _distribution_version

from finalytics._finalytics import version

__version__: str = _distribution_version("finalytics")
"""The version of this package, in Python's version format (for example ``1.0.0``)."""

__all__ = ["__version__", "version"]
