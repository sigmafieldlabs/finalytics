"""Runs the Python examples in docs/methods and in the package's README, so that every output
shown there is the engine's."""

import doctest
import pathlib
import re

import pytest

from conftest import ROOT

PAGES = [
    *sorted(p for p in (ROOT / "docs" / "methods").glob("*.md") if p.name != "README.md"),
    ROOT / "bindings" / "python" / "README.md",
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
