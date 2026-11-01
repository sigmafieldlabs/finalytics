"""Runs the Python examples in docs/methods, so that every output shown there is the engine's."""

import doctest
import pathlib
import re

import pytest

from conftest import ROOT

PAGES = sorted(p for p in (ROOT / "docs" / "methods").glob("*.md") if p.name != "README.md")
BLOCK = re.compile(r"```python\n(.*?)```", re.DOTALL)


@pytest.mark.parametrize("page", PAGES, ids=lambda p: p.name)
def test_examples_run(page: pathlib.Path):
    blocks = BLOCK.findall(page.read_text(encoding="utf-8"))
    assert blocks, f"{page.name} has no Python example"
    parser = doctest.DocTestParser()
    runner = doctest.DocTestRunner(optionflags=doctest.NORMALIZE_WHITESPACE)
    test = parser.get_doctest("\n".join(blocks), {}, page.name, str(page), 0)
    result = runner.run(test, out=lambda text: print(text, end=""))
    assert result.failed == 0, f"{result.failed} of {result.attempted} examples in {page.name} failed"
