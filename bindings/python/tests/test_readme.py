"""Runs the package README's examples in order, as a reader would, in a folder that holds the
portfolio dataset's files. Each example that ends in ``show()`` must open one page, and each
that the README follows with a text block must print that text."""

import contextlib
import io
import os
import pathlib
import re
import shutil
import webbrowser

from conftest import ROOT

README = ROOT / "bindings" / "python" / "README.md"
DATASET = ROOT / "crates" / "finalytics" / "data" / "portfolio"
BLOCK = re.compile(r"```(\w+)\n(.*?)```", re.DOTALL)


def examples(text: str) -> list[tuple[str, str | None]]:
    """Each Python block, with the text block that shows its output, if one follows it."""
    found: list[tuple[str, str | None]] = []
    for language, body in BLOCK.findall(text):
        if language == "python":
            found.append((body, None))
        elif language == "text" and found and found[-1][1] is None:
            found[-1] = (found[-1][0], body)
    return found


def test_the_examples_run_and_show_their_reports(tmp_path, monkeypatch):
    blocks = examples(README.read_text(encoding="utf-8"))
    assert len(blocks) >= 5
    for source in DATASET.glob("*.csv"):
        shutil.copy(source, tmp_path / source.name)
    opened: list[str] = []
    monkeypatch.setattr(webbrowser, "open", lambda url, *args, **kwargs: opened.append(url) or True)
    namespace: dict = {}
    here = os.getcwd()
    os.chdir(tmp_path)
    try:
        for code, expected in blocks:
            printed = io.StringIO()
            shown = len(opened)
            with contextlib.redirect_stdout(printed):
                exec(compile(code, str(README), "exec"), namespace)  # noqa: S102
            if expected is not None:
                assert printed.getvalue() == expected
            if ".show()" in code:
                assert len(opened) == shown + 1, code
    finally:
        os.chdir(here)
    for url in opened:
        page = pathlib.Path(url.removeprefix("file://"))
        assert page.read_text(encoding="utf-8").startswith("<!doctype html>")
        page.unlink()
