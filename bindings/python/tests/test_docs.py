"""Runs the Python examples in docs/methods, the data guide and the migration guide, so that every
output shown there is the engine's. The package's README is run by test_readme.py."""

import doctest
import pathlib
import re

import pytest

from conftest import ROOT

PAGES = [
    *sorted(p for p in (ROOT / "docs" / "methods").glob("*.md") if p.name != "README.md"),
    ROOT / "docs" / "migrating-from-0.9.md",
    ROOT / "docs" / "data.md",
]
BLOCK = re.compile(r"```python\n(.*?)```", re.DOTALL)


@pytest.mark.parametrize("page", PAGES, ids=lambda p: p.name)
def test_examples_run(page: pathlib.Path):
    blocks = BLOCK.findall(page.read_text(encoding="utf-8"))
    assert blocks, f"{page.name} has no Python example"
    parser = doctest.DocTestParser()
    # Not verbose: doctest's verbose mode follows "-v" on the command line, and pytest's -v would
    # otherwise mix the report into the examples' captured output.
    runner = doctest.DocTestRunner(verbose=False, optionflags=doctest.NORMALIZE_WHITESPACE)
    test = parser.get_doctest("\n".join(blocks), {}, page.name, str(page), 0)
    report: list[str] = []
    result = runner.run(test, out=report.append)
    assert result.failed == 0, "".join(report)


def _literal_blocks(doc: str) -> list[str]:
    """The literal blocks of a docstring: the indented lines after a line ending in ``::``."""
    import inspect
    import textwrap

    lines = inspect.cleandoc(doc).splitlines()
    found, i = [], 0
    while i < len(lines):
        if lines[i].rstrip().endswith("::"):
            i += 1
            while i < len(lines) and not lines[i].strip():
                i += 1
            start = i
            while i < len(lines) and (not lines[i].strip() or lines[i].startswith(" ")):
                i += 1
            code = textwrap.dedent("\n".join(lines[start:i])).strip()
            if code:
                found.append(code)
        else:
            i += 1
    return found


def _docstrings():
    import inspect

    import finalytics as fl
    import finalytics.assets
    import finalytics.pandas
    import finalytics.portfolio
    import finalytics.screening

    modules = [fl, fl.assets, fl.portfolio, fl.screening, fl.datasets, fl.opt, fl.risk, fl.reports, fl.performance, fl.pandas]
    seen = set()
    for module in modules:
        if module.__doc__:
            yield module.__name__, module.__doc__
        for name, obj in vars(module).items():
            if name.startswith("_") or id(obj) in seen or getattr(obj, "__module__", "") != module.__name__:
                continue
            seen.add(id(obj))
            if (inspect.isclass(obj) or inspect.isfunction(obj)) and obj.__doc__:
                yield f"{module.__name__}.{name}", obj.__doc__
            if inspect.isclass(obj):
                for attr, member in vars(obj).items():
                    target = member.fget if isinstance(member, property) else member
                    doc = getattr(target, "__doc__", None)
                    if doc and not attr.startswith("_") and callable(getattr(target, "__call__", None)):
                        yield f"{module.__name__}.{name}.{attr}", doc


EXAMPLES = [
    (where, code)
    for where, doc in _docstrings()
    for code in _literal_blocks(doc)
    # Shell commands and file layouts are not Python.
    if not code.startswith(("pip ", "$", "date,", "id,")) and ("\n" in code or "(" in code or "=" in code)
]


@pytest.mark.parametrize(("where", "code"), EXAMPLES, ids=[f"{w}-{k}" for k, (w, _) in enumerate(EXAMPLES)])
def test_docstring_examples_run(where, code, tmp_path, monkeypatch):
    # Each runs as written, in a folder holding the sample household's files, with nothing shown:
    # show() hands its page to a handler that keeps it nowhere, which works on Pyodide too, where
    # there is no browser to open.
    import finalytics as fl

    for name in fl.datasets.FILES:
        if name.startswith("portfolio/"):
            (tmp_path / name.split("/")[-1]).write_text(fl.datasets.file(name), encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr("finalytics._printing._show_handler", lambda page: None)
    exec(compile(code, where, "exec"), {})  # noqa: S102
