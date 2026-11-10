#!/usr/bin/env python3
"""Run the guides' examples and check the guides, in docs/guides and docs/examples.

    python3 scripts/guides.py            run every example in each of its languages, and check
                                         that they print the same, that the output is the one
                                         recorded, and that the pages are sound
    python3 scripts/guides.py --record   run them and record their output, and write the list of
                                         Rust examples into docs/examples/Cargo.toml

    --languages python,javascript   only these languages (python, javascript, rust)
    --python PATH                   the Python that runs the examples, with finalytics installed;
                                    the one running this script by default
    --page NAME                     only the examples of this page

Python runs with the package installed in that interpreter, JavaScript with the npm package built
in bindings/wasm/pkg (bindings/wasm/build.sh), and Rust with the examples built by cargo. Each
example runs in a folder of its own that holds the household dataset's CSV files, as a reader's
working folder would, with a browser that opens nothing.

The pages are checked for what docs/guides/README.md asks of them: every example they show
exists, every example belongs to a page, every page is in contents.json, the code fits 80
columns, and every decimal, percentage or amount in the prose is one the examples print or
are given.
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from decimal import Decimal, InvalidOperation
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GUIDES = ROOT / "docs" / "guides"
EXAMPLES = ROOT / "docs" / "examples"
CARGO = EXAMPLES / "Cargo.toml"
PACKAGE = "finalytics-guides"
DATA = ROOT / "crates" / "finalytics" / "data" / "portfolio"
NPM = ROOT / "bindings" / "wasm" / "pkg"
LANGUAGES = {"python": ".py", "javascript": ".js", "rust": ".rs"}
WIDTH = 80

FRONT_MATTER = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
FENCE = re.compile(r"^(```|~~~).*?^\1[ \t]*$", re.DOTALL | re.MULTILINE)
INLINE_CODE = re.compile(r"`[^`\n]+`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
# An example shown on a page: a paragraph that is one link to the example's Python, JavaScript or
# Rust file, such as [optimise.py](../../examples/optimise-a-portfolio/optimise.py).
EXAMPLE = re.compile(r"^\[[^\]\n]+\]\(\.\./\.\./examples/([a-z0-9-]+)/([a-z0-9-]+)\.(py|js|rs)\)[ \t]*$", re.MULTILINE)
# A decimal, a percentage or an amount of money: the numbers that results are made of. Whole
# numbers alone (15 methods, 2016) are not checked.
NUMBER = re.compile(r"(?<![\w.])[$€£]?-?\d[\d,]*(?:\.\d+)?%?(?![\w])")
# Version numbers (1.0.0, 1.0.0-rc.6) are not results.
VERSION = re.compile(r"(?<![\w.])\d+\.\d+\.\d+(?:-[\w.]+)?")


@dataclass
class Example:
    page: str
    name: str
    files: dict[str, Path] = field(default_factory=dict)

    @property
    def key(self) -> str:
        return f"{self.page}/{self.name}"

    @property
    def recorded(self) -> Path:
        return EXAMPLES / self.page / f"{self.name}.out"

    @property
    def cargo_name(self) -> str:
        return f"{self.page}-{self.name}"


@dataclass
class Page:
    path: Path
    slug: str
    meta: dict[str, object]
    text: str
    shown: list[tuple[str, str]]


def examples() -> dict[str, Example]:
    found: dict[str, Example] = {}
    for folder in sorted(p for p in EXAMPLES.iterdir() if p.is_dir() and not p.name.startswith((".", "target"))):
        for path in sorted(folder.iterdir()):
            language = next((lang for lang, suffix in LANGUAGES.items() if path.suffix == suffix), None)
            if language is None:
                continue
            example = found.setdefault(f"{folder.name}/{path.stem}", Example(folder.name, path.stem))
            example.files[language] = path
    return found


def front_matter(text: str) -> tuple[dict[str, object], str]:
    """A page's front matter, `key: value` lines with lists as `[a, b]`, and the rest of it."""
    match = FRONT_MATTER.match(text)
    if not match:
        return {}, text
    meta: dict[str, object] = {}
    for line in match.group(1).splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        key, _, value = line.partition(":")
        value = value.strip()
        if value.startswith("[") and value.endswith("]"):
            meta[key.strip()] = [v.strip().strip("\"'") for v in value[1:-1].split(",") if v.strip()]
        else:
            meta[key.strip()] = value.strip("\"'")
    return meta, text[match.end():]


def pages() -> list[Page]:
    found = []
    for path in sorted(GUIDES.glob("*/*.md")):
        meta, body = front_matter(path.read_text(encoding="utf-8"))
        shown = [(m.group(1), m.group(2)) for m in EXAMPLE.finditer(body)]
        found.append(Page(path, path.stem, meta, body, shown))
    return found


def prose(text: str) -> str:
    """A page's prose: without its code, comments and link targets."""
    for pattern in (FENCE, COMMENT, INLINE_CODE, LINK_TARGET):
        text = pattern.sub(" ", text)
    return text


def value(token: str) -> tuple[Decimal, int] | None:
    """A number as written, such as "6.4%" or "$1,250.50", as a fraction for a percentage, with
    the decimal places it is written to (two more for a percentage)."""
    text = token.lstrip("$€£").replace(",", "")
    percent = text.endswith("%")
    text = text.rstrip("%")
    try:
        number = Decimal(text)
    except InvalidOperation:
        return None
    places = len(text.partition(".")[2])
    return (number / 100, places + 2) if percent else (number, places)


def known_numbers(texts: list[str]) -> set[Decimal]:
    """Every number in the examples' code and output, each also as a fraction if it is a
    percentage."""
    found: set[Decimal] = set()
    for text in texts:
        text = text.replace("−", "-")
        for token in NUMBER.findall(text):
            parsed = value(token)
            if parsed:
                found.add(parsed[0])
                found.add(-parsed[0])
    return found


def quoted(number: Decimal, places: int, known: set[Decimal]) -> bool:
    """Whether `number`, written to `places` decimal places, is one of `known` as written or
    rounded."""
    step = Decimal(1).scaleb(-places)
    return any(k.quantize(step) == number for k in known if abs(k) < 10**15)


def check_page(page: Page, by_key: dict[str, Example], outputs: dict[str, str]) -> list[str]:
    problems = []
    where = page.path.relative_to(ROOT)
    if not page.text.lstrip().startswith("# "):
        problems.append(f"{where}: starts without a title (# ...)")
    if "description" not in page.meta:
        problems.append(f"{where}: has no description in its front matter")
    for folder, name in page.shown:
        if folder != page.slug:
            problems.append(f"{where}: shows {folder}/{name}, which belongs to the page {folder}")
        elif f"{folder}/{name}" not in by_key:
            problems.append(f"{where}: shows {folder}/{name}, which docs/examples does not have")
    # Numbers in the prose come from the examples: printed by them, or given to them.
    texts = []
    for folder, name in page.shown:
        example = by_key.get(f"{folder}/{name}")
        if example is None:
            continue
        texts += [path.read_text(encoding="utf-8") for path in example.files.values()]
        if example.key in outputs:
            texts.append(outputs[example.key])
        elif example.recorded.exists():
            texts.append(example.recorded.read_text(encoding="utf-8"))
    known = known_numbers(texts)
    allowed = {str(n) for n in page.meta.get("numbers", []) or []}
    text = VERSION.sub(" ", prose(page.text).replace("−", "-"))
    for token in NUMBER.findall(text):
        if not re.search(r"[.%$€£]", token) or token in allowed:
            continue
        parsed = value(token)
        if parsed and not quoted(parsed[0], parsed[1], known):
            problems.append(
                f"{where}: the prose says {token}, which no example of the page prints or is given"
                " (quote a number as an example prints it, or list it under `numbers` in the front"
                " matter if it is not a result)"
            )
    return problems


def check_layout(found: dict[str, Example], all_pages: list[Page]) -> list[str]:
    problems = []
    shown = {f"{f}/{n}" for page in all_pages for f, n in page.shown}
    for key, example in found.items():
        if key not in shown:
            problems.append(f"docs/examples/{key}: no page shows it")
        for path in example.files.values():
            for k, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if len(line) > WIDTH:
                    problems.append(f"{path.relative_to(ROOT)}:{k}: wider than {WIDTH} columns")
    contents = json.loads((GUIDES / "contents.json").read_text(encoding="utf-8"))
    listed = [f"{group['folder']}/{slug}" for group in contents["groups"] for slug in group["pages"]]
    files = [f"{p.path.parent.name}/{p.slug}" for p in all_pages]
    for missing in sorted(set(files) - set(listed)):
        problems.append(f"docs/guides/{missing}.md is not in docs/guides/contents.json")
    for missing in sorted(set(listed) - set(files)):
        problems.append(f"docs/guides/contents.json lists {missing}, which has no page")
    slugs = [p.slug for p in all_pages]
    for slug in sorted({s for s in slugs if slugs.count(s) > 1}):
        problems.append(f"two pages are named {slug}: a page's name is its address")
    return problems


def cargo_examples(found: dict[str, Example]) -> str:
    entries = [
        f'[[example]]\nname = "{e.cargo_name}"\npath = "{e.page}/{e.name}.rs"\n'
        for e in found.values()
        if "rust" in e.files
    ]
    return "\n".join(entries)


MARK_START = "# The examples, written by scripts/guides.py --record.\n"


def write_cargo(found: dict[str, Example]) -> bool:
    """Writes the Rust examples into docs/examples/Cargo.toml; whether it changed."""
    text = CARGO.read_text(encoding="utf-8")
    head = text.split(MARK_START)[0].rstrip("\n") + "\n\n"
    new = head + MARK_START + "\n" + cargo_examples(found)
    if new != text:
        CARGO.write_text(new, encoding="utf-8")
        return True
    return False


def cargo_current(found: dict[str, Example]) -> bool:
    text = CARGO.read_text(encoding="utf-8")
    return MARK_START in text and text.split(MARK_START)[1].strip() == cargo_examples(found).strip()


def scratch() -> Path:
    """A folder that holds the household dataset's files, with the npm package installed."""
    here = Path(tempfile.mkdtemp(prefix="finalytics-guide-"))
    for data in DATA.glob("*.csv"):
        shutil.copy(data, here)
    (here / "package.json").write_text('{ "type": "module" }\n', encoding="utf-8")
    (here / "node_modules").mkdir()
    if NPM.exists():
        os.symlink(NPM, here / "node_modules" / "finalytics", target_is_directory=True)
    return here


def run(example: Example, language: str, python: str, binaries: Path) -> tuple[str | None, str]:
    """Runs one example; its output, or None and why it failed."""
    here = scratch()
    try:
        source = example.files[language]
        if language == "python":
            command = [python, str(source)]
        elif language == "javascript":
            target = here / source.name
            shutil.copy(source, target)
            command = ["node", str(target)]
        else:
            command = [str(binaries / example.cargo_name)]
        env = {**os.environ, "BROWSER": "true", "PYTHONIOENCODING": "utf-8", "NO_COLOR": "1"}
        found = subprocess.run(command, cwd=here, capture_output=True, text=True, encoding="utf-8", env=env)
        if found.returncode != 0:
            return None, found.stderr.strip() or f"exit status {found.returncode}"
        return found.stdout, ""
    finally:
        shutil.rmtree(here, ignore_errors=True)


def build_rust() -> Path:
    subprocess.run(
        ["cargo", "build", "--quiet", "-p", PACKAGE, "--examples"],
        cwd=ROOT,
        check=True,
    )
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    return target / "debug" / "examples"


def diff(a: str, b: str, a_name: str, b_name: str) -> str:
    lines = difflib.unified_diff(
        a.splitlines(keepends=True), b.splitlines(keepends=True), a_name, b_name, n=1
    )
    return "".join(list(lines)[:40])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--languages", default=",".join(LANGUAGES))
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--page")
    args = parser.parse_args()
    if os.sep in args.python:
        args.python = str(Path(args.python).absolute())
    languages = [lang.strip() for lang in args.languages.split(",") if lang.strip()]
    unknown = sorted(set(languages) - set(LANGUAGES))
    if unknown:
        parser.error(f"unknown languages: {', '.join(unknown)} (expected {', '.join(LANGUAGES)})")
    if args.record and set(languages) != set(LANGUAGES):
        parser.error("--record runs every language, so that the output recorded is theirs")

    found = examples()
    all_pages = pages()
    problems = check_layout(found, all_pages)
    if args.record:
        if write_cargo(found):
            print(f"wrote the Rust examples into {CARGO.relative_to(ROOT)}")
    elif not cargo_current(found):
        problems.append(f"{CARGO.relative_to(ROOT)} does not list the Rust examples as they are: run scripts/guides.py --record")

    chosen = [e for e in found.values() if args.page in (None, e.page)]
    binaries = build_rust() if "rust" in languages and any("rust" in e.files for e in chosen) else Path()
    outputs: dict[str, str] = {}
    for example in chosen:
        ran: dict[str, str] = {}
        for language in languages:
            if language not in example.files:
                continue
            output, error = run(example, language, args.python, binaries)
            if output is None:
                problems.append(f"{example.files[language].relative_to(ROOT)} failed:\n{error}")
            else:
                ran[language] = output
        if not ran:
            continue
        first, text = next(iter(ran.items()))
        for language, output in ran.items():
            if output != text:
                problems.append(
                    f"docs/examples/{example.key}: {language} prints otherwise than {first}:\n"
                    + diff(text, output, first, language)
                )
        outputs[example.key] = text
        if args.record:
            if len(ran) == len(example.files):
                example.recorded.write_text(text, encoding="utf-8")
        elif not example.recorded.exists():
            problems.append(f"docs/examples/{example.key} has no recorded output: run scripts/guides.py --record")
        else:
            recorded = example.recorded.read_text(encoding="utf-8")
            for language, output in ran.items():
                if output != recorded:
                    problems.append(
                        f"docs/examples/{example.key}: {language} prints otherwise than recorded:\n"
                        + diff(recorded, output, "recorded", language)
                    )
    for stale in sorted(EXAMPLES.glob("*/*.out")):
        key = f"{stale.parent.name}/{stale.stem}"
        if key not in found:
            problems.append(f"{stale.relative_to(ROOT)} is the output of an example that does not exist")

    by_key = {e.key: e for e in found.values()}
    for page in all_pages:
        problems += check_page(page, by_key, outputs)

    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    ran_count = len(chosen)
    if problems:
        return 1
    verb = "recorded" if args.record else "checked"
    print(f"guides {verb}: {len(all_pages)} pages, {ran_count} examples in {', '.join(languages)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
