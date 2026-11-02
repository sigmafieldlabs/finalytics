"""Runs the package README's examples as a reader would, and checks that each prints what the
README shows after it."""

import contextlib
import io
import os
import re

from conftest import ROOT

README = ROOT / "bindings" / "python" / "README.md"
BLOCK = re.compile(r"```(\w+)\n(.*?)```", re.DOTALL)
PRICES = "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n"


def examples(text: str) -> list[tuple[str, str | None]]:
    """Each Python block, with the text block that shows its output, if one follows it."""
    found: list[tuple[str, str | None]] = []
    for language, body in BLOCK.findall(text):
        if language == "python":
            found.append((body, None))
        elif language == "text" and found and found[-1][1] is None:
            found[-1] = (found[-1][0], body)
    return found


def test_the_examples_print_what_the_readme_shows(tmp_path):
    text = README.read_text(encoding="utf-8")
    blocks = examples(text)
    assert len(blocks) >= 3 and all(output is not None for _, output in blocks[:1])
    # The files the examples read: the transactions as the README shows them.
    csv = re.search(r"```csv\n(.*?)```", text, re.DOTALL)
    assert csv, "the README shows a transactions file"
    (tmp_path / "transactions.csv").write_text(csv.group(1), encoding="utf-8")
    (tmp_path / "prices.csv").write_text(PRICES, encoding="utf-8")
    namespace: dict = {}
    here = os.getcwd()
    os.chdir(tmp_path)
    try:
        for code, expected in blocks:
            printed = io.StringIO()
            with contextlib.redirect_stdout(printed):
                exec(compile(code, str(README), "exec"), namespace)  # noqa: S102
            if expected is not None:
                assert printed.getvalue() == expected
    finally:
        os.chdir(here)
    assert (tmp_path / "report.html").read_text(encoding="utf-8").startswith("<!doctype html>")
