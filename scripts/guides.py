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
working folder would. show() hands its page to a handler that keeps nothing, as the site's pages
hand it to the output panel; in Rust, which has no handler, the browser opens nothing.

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
from decimal import ROUND_HALF_DOWN, ROUND_HALF_UP, Decimal, InvalidOperation
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GUIDES = ROOT / "docs" / "guides"
EXAMPLES = ROOT / "docs" / "examples"
CARGO = EXAMPLES / "Cargo.toml"
PACKAGE = "finalytics-guides"
DATA = ROOT / "crates" / "finalytics" / "data" / "portfolio"
NPM = ROOT / "bindings" / "wasm" / "pkg"
LANGUAGES = {"python": ".py", "javascript": ".js", "rust": ".rs"}
SUFFIXES = {suffix: language for language, suffix in LANGUAGES.items()}
WIDTH = 80
# Folders in docs/examples that are not pages' examples.
NOT_PAGES = {"target", "node_modules"}

FRONT_MATTER = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
FENCE = re.compile(r"^(```|~~~).*?^\1[ \t]*$", re.DOTALL | re.MULTILINE)
INLINE_CODE = re.compile(r"`[^`\n]+`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
# An example shown on a page: a paragraph that is one link to one of the example's files, such
# as [optimise.py](../../examples/optimise-a-portfolio/optimise.py).
EXAMPLE = re.compile(
    r"^\[[^\]\n]+\]\(\.\./\.\./examples/([a-z0-9-]+)/([a-z0-9-]+)(\.py|\.js|\.rs)\)[ \t]*$", re.MULTILINE
)
# A number in full, whatever unit follows it (2.8x, $1.2m, 1.52bn, 2.5 MiB): a decimal, a
# percentage or an amount of money is checked; a whole number alone (15 methods, 2016) is not.
NUMBER = re.compile(r"(?<![\w.])([$€£]?)(-?\d[\d,]*(?:\.\d+)?)(%?)")
# Version numbers (1.0.0, 1.0.0-rc.6, Python 3.10) are not results.
VERSION = re.compile(
    r"(?<![\w.])\d+\.\d+\.\d+(?:-[\w.]+)?"
    r"|(?i:\b(?:python|node(?:\.js)?|rust|pyodide|typescript|finalytics|version)\s+)\d+(?:\.\d+)+"
)


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
        return f"{self.page}--{self.name}"


@dataclass
class Page:
    path: Path
    slug: str
    meta: dict[str, object]
    text: str
    shown: list[tuple[str, str, str]]


def examples() -> dict[str, Example]:
    found: dict[str, Example] = {}
    folders = (p for p in EXAMPLES.iterdir() if p.is_dir() and p.name not in NOT_PAGES and not p.name.startswith("."))
    for folder in sorted(folders):
        for path in sorted(folder.iterdir()):
            language = SUFFIXES.get(path.suffix)
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
        shown = [(m.group(1), m.group(2), m.group(3)) for m in EXAMPLE.finditer(body)]
        found.append(Page(path, path.stem, meta, body, shown))
    return found


def prose(text: str) -> str:
    """A page's prose: without its code, comments and link targets."""
    for pattern in (FENCE, COMMENT, INLINE_CODE, LINK_TARGET):
        text = pattern.sub(" ", text)
    return text


def value(match: re.Match[str]) -> tuple[Decimal, int, bool] | None:
    """A number as written, such as "6.4%" or "$1,250.50": its value, the decimal places it is
    written to, and whether it is a percentage."""
    digits = match.group(2).replace(",", "")
    try:
        number = Decimal(digits)
    except InvalidOperation:
        return None
    return number, len(digits.partition(".")[2]), bool(match.group(3))


def known_numbers(texts: list[str]) -> tuple[set[Decimal], set[Decimal]]:
    """The numbers in `texts`: those written as percentages, and the others."""
    percentages: set[Decimal] = set()
    others: set[Decimal] = set()
    for text in texts:
        for match in NUMBER.finditer(text.replace("\u2212", "-")):
            parsed = value(match)
            if parsed:
                number, _, percent = parsed
                (percentages if percent else others).update({number, -number})
    return percentages, others


def quoted(number: Decimal, places: int, known: set[Decimal]) -> bool:
    """Whether `number`, written to `places` decimal places, is one of `known` as written or
    rounded: on a tie, rounded either way."""
    step = Decimal(1).scaleb(-places)
    return any(
        k.quantize(step, rounding=mode) == number
        for k in known
        if abs(k) < 10**15
        for mode in (ROUND_HALF_UP, ROUND_HALF_DOWN)
    )


def check_page(page: Page, by_key: dict[str, Example], outputs: dict[str, str]) -> list[str]:
    problems = []
    where = page.path.relative_to(ROOT)
    if not page.text.lstrip().startswith("# "):
        problems.append(f"{where}: starts without a title (# ...)")
    if "description" not in page.meta:
        problems.append(f"{where}: has no description in its front matter")
    for folder, name, suffix in page.shown:
        example = by_key.get(f"{folder}/{name}")
        if folder != page.slug:
            problems.append(f"{where}: shows {folder}/{name}, which belongs to the page {folder}")
        elif example is None:
            problems.append(f"{where}: shows {folder}/{name}, which docs/examples does not have")
        elif SUFFIXES[suffix] not in example.files:
            problems.append(f"{where}: links to {folder}/{name}{suffix}, which does not exist")
    # Numbers in the prose come from the examples. A percentage is one an example prints as a
    # percentage, or a fraction it is given (max_weight=0.40 for 40%); any other number is one it
    # prints or is given as it is.
    printed: list[str] = []
    given: list[str] = []
    for folder, name, _ in page.shown:
        example = by_key.get(f"{folder}/{name}")
        if example is None:
            continue
        given += [path.read_text(encoding="utf-8") for path in example.files.values()]
        if example.key in outputs:
            printed.append(outputs[example.key])
        elif example.recorded.exists():
            printed.append(example.recorded.read_text(encoding="utf-8"))
    printed_percentages, printed_others = known_numbers(printed)
    given_percentages, given_others = known_numbers(given)
    as_percentages = printed_percentages | given_percentages | {n * 100 for n in given_others}
    as_others = printed_others | given_others
    listed = page.meta.get("numbers") or []
    allowed = {str(n) for n in (listed if isinstance(listed, list) else [listed])}
    text = VERSION.sub(" ", prose(page.text).replace("\u2212", "-"))
    for match in NUMBER.finditer(text):
        token = match.group(0)
        if not (match.group(1) or match.group(3) or "." in match.group(2)) or token in allowed:
            continue
        parsed = value(match)
        if parsed is None:
            continue
        number, places, percent = parsed
        if not quoted(number, places, as_percentages if percent else as_others):
            problems.append(
                f"{where}: the prose says {token}, which no example of the page prints or is given"
                " (quote a number as an example prints it, or list it under `numbers` in the front"
                " matter if it is not a result)"
            )
    return problems


def check_layout(found: dict[str, Example], all_pages: list[Page]) -> list[str]:
    problems = []
    shown = {f"{f}/{n}" for page in all_pages for f, n, _ in page.shown}
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


MARK_START = "# The examples, written by scripts/guides.py --record.\n"


def cargo_examples(found: dict[str, Example]) -> str:
    entries = [
        f'[[example]]\nname = "{e.cargo_name}"\npath = "{e.page}/{e.name}.rs"\n'
        for e in found.values()
        if "rust" in e.files
    ]
    return "\n".join(entries)


def write_cargo(found: dict[str, Example]) -> list[str]:
    """Writes the Rust examples into docs/examples/Cargo.toml; the problems, if it cannot."""
    if not any("rust" in e.files for e in found.values()):
        return [f"{CARGO.relative_to(ROOT)} needs a Rust example: cargo refuses a package without one"]
    text = CARGO.read_text(encoding="utf-8")
    head = text.split(MARK_START)[0].rstrip("\n") + "\n\n"
    new = head + MARK_START + "\n" + cargo_examples(found)
    if new != text:
        CARGO.write_text(new, encoding="utf-8")
        print(f"wrote the Rust examples into {CARGO.relative_to(ROOT)}")
    return []


def cargo_current(found: dict[str, Example]) -> bool:
    text = CARGO.read_text(encoding="utf-8")
    return MARK_START in text and text.split(MARK_START)[1].strip() == cargo_examples(found).strip()


def build_rust() -> tuple[dict[str, Path], str]:
    """Builds the Rust examples: each one's program by name, or why they did not build."""
    try:
        found = subprocess.run(
            ["cargo", "build", "--quiet", "-p", PACKAGE, "--examples", "--message-format=json"],
            cwd=ROOT, capture_output=True, text=True, encoding="utf-8",
        )
    except OSError as error:
        return {}, f"cannot run cargo: {error}"
    if found.returncode != 0:
        messages = [json.loads(line) for line in found.stdout.splitlines() if line.startswith("{")]
        rendered = [m["message"]["rendered"] for m in messages if m.get("reason") == "compiler-message"]
        return {}, "".join(rendered) or found.stderr
    programs = {}
    for line in found.stdout.splitlines():
        message = json.loads(line) if line.startswith("{") else {}
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            programs[message["target"]["name"]] = Path(message["executable"])
    return programs, ""


def scratch() -> Path:
    """A folder that holds the household dataset's files, with the npm package installed."""
    here = Path(tempfile.mkdtemp(prefix="finalytics-guide-"))
    for data in DATA.glob("*.csv"):
        shutil.copy(data, here)
    (here / "package.json").write_text('{ "type": "module" }\n', encoding="utf-8")
    (here / "node_modules").mkdir()
    return here


def install_npm(here: Path) -> None:
    target = here / "node_modules" / "finalytics"
    try:
        os.symlink(NPM, target, target_is_directory=True)
    except OSError:
        # Windows allows links only with privileges; a copy will do.
        shutil.copytree(NPM, target)


# Each example runs as a reader runs it, with show() handing its page to a handler that keeps it
# nowhere, as the site's pages hand it to their output panel.
PYTHON_RUNNER = """\
import runpy, sys
import finalytics
finalytics.set_show_handler(lambda page: None)
path = sys.argv[1]
sys.argv = [path]
runpy.run_path(path, run_name="__main__")
"""
JAVASCRIPT_RUNNER = """\
import {{ setShowHandler }} from "finalytics";
setShowHandler(() => {{}});
await import("./{name}");
"""


def run(example: Example, language: str, python: str, programs: dict[str, Path]) -> tuple[str | None, str]:
    """Runs one example; its output, or None and why it failed."""
    here = scratch()
    try:
        source = example.files[language]
        if language == "python":
            command = [python, "-c", PYTHON_RUNNER, str(source)]
        elif language == "javascript":
            install_npm(here)
            shutil.copy(source, here / source.name)
            runner = here / "run-example.js"
            runner.write_text(JAVASCRIPT_RUNNER.format(name=source.name), encoding="utf-8")
            command = ["node", str(runner)]
        else:
            program = programs.get(example.cargo_name)
            if program is None:
                return None, f"cargo built no program {example.cargo_name}"
            command = [str(program)]
        browser = "cmd /c rem" if os.name == "nt" else "true"
        env = {**os.environ, "BROWSER": browser, "PYTHONIOENCODING": "utf-8", "NO_COLOR": "1"}
        try:
            found = subprocess.run(command, cwd=here, capture_output=True, text=True, encoding="utf-8", env=env)
        except OSError as error:
            return None, f"cannot run {command[0]}: {error}"
        if found.returncode != 0:
            return None, found.stderr.strip() or f"exit status {found.returncode}"
        return found.stdout, ""
    finally:
        shutil.rmtree(here, ignore_errors=True)


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
    if os.sep in args.python or (os.altsep and os.altsep in args.python):
        args.python = str(Path(args.python).absolute())
    languages = [lang.strip() for lang in args.languages.split(",") if lang.strip()]
    unknown = sorted(set(languages) - set(LANGUAGES))
    if unknown:
        parser.error(f"unknown languages: {', '.join(unknown)} (expected {', '.join(LANGUAGES)})")
    if args.record and set(languages) != set(LANGUAGES):
        parser.error("--record runs every language, so that the output recorded is theirs")

    found = examples()
    all_pages = pages()
    if args.page and args.page not in {e.page for e in found.values()}:
        parser.error(f"no page {args.page!r} has examples in docs/examples")
    problems = check_layout(found, all_pages)
    names = [e.cargo_name for e in found.values() if "rust" in e.files]
    for name in sorted({n for n in names if names.count(n) > 1}):
        problems.append(f"two Rust examples would both be the program {name}")
    rust_listed = True
    if args.record:
        problems += write_cargo(found)
    elif not cargo_current(found):
        rust_listed = False
        problems.append(f"{CARGO.relative_to(ROOT)} does not list the Rust examples as they are: run scripts/guides.py --record")

    chosen = [e for e in found.values() if args.page in (None, e.page)]
    programs: dict[str, Path] = {}
    if "rust" in languages and rust_listed and any("rust" in e.files for e in chosen):
        programs, error = build_rust()
        if error:
            problems.append(f"the Rust examples did not build:\n{error}")
    outputs: dict[str, str] = {}
    for example in chosen:
        ran: dict[str, str] = {}
        for language in languages:
            if language not in example.files or (language == "rust" and not programs):
                continue
            output, error = run(example, language, args.python, programs)
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
        if f"{stale.parent.name}/{stale.stem}" not in found:
            problems.append(f"{stale.relative_to(ROOT)} is the output of an example that does not exist")

    by_key = {e.key: e for e in found.values()}
    for page in all_pages:
        problems += check_page(page, by_key, outputs)

    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if problems:
        return 1
    verb = "recorded" if args.record else "checked"
    print(f"guides {verb}: {len(all_pages)} pages, {len(chosen)} examples in {', '.join(languages)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
