#!/usr/bin/env python3
"""Check that version numbers and licence copies agree across the repository, that the READMEs'
code fits 80 columns, and that the Rust programs in README.md are the crate's examples as they
are.

With --run-examples, also run those examples and check that each prints what README.md shows
after it.
"""

import json
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
READMES = ["README.md", "bindings/python/README.md", "bindings/wasm/README.md"]
BLOCK = re.compile(r"```([\w,]*)\n(.*?)```", re.DOTALL)
EXAMPLES = ROOT / "crates/finalytics/examples"


def rust_examples(readme: str) -> list[tuple[str, str | None]]:
    """The README's Rust programs, each with the output shown after it, if any."""
    blocks = BLOCK.findall(readme)
    found = []
    for k, (language, body) in enumerate(blocks):
        if language.startswith("rust") and "fn main" in body:
            following = blocks[k + 1] if k + 1 < len(blocks) else ("", "")
            found.append((body, following[1] if following[0] == "text" else None))
    return found


def readme_problems(run_examples: bool) -> list[str]:
    problems = []
    for name in READMES:
        for _, body in BLOCK.findall((ROOT / name).read_text(encoding="utf-8")):
            for line in body.splitlines():
                if len(line) > 80:
                    problems.append(f"{name}: a line of code is wider than 80 columns: {line}")
    examples = {path.read_text(encoding="utf-8"): path for path in sorted(EXAMPLES.glob("*.rs"))}
    for code, output in rust_examples((ROOT / "README.md").read_text(encoding="utf-8")):
        path = examples.get(code)
        if path is None:
            first = code.splitlines()[0] if code else ""
            problems.append(f"README.md shows a program that is not in {EXAMPLES.relative_to(ROOT)}: {first}")
            continue
        if not run_examples or output is None:
            continue
        with tempfile.TemporaryDirectory() as here:
            found = subprocess.run(
                ["cargo", "run", "--quiet", "--manifest-path", str(ROOT / "Cargo.toml"), "-p", "finalytics",
                 "--example", path.stem],
                cwd=here, capture_output=True, text=True,
            )
        if found.returncode != 0:
            problems.append(f"the example {path.name} failed: {found.stderr}")
        elif found.stdout != output:
            problems.append(f"the example {path.name} prints\n{found.stdout}\nbut README.md shows\n{output}")
    return problems


def main() -> int:
    problems = []
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    version = workspace["workspace"]["package"]["version"]

    package = json.loads((ROOT / "bindings/wasm/package.json").read_text())
    if package["version"] != version:
        problems.append(f"bindings/wasm/package.json has version {package['version']}, Cargo.toml has {version}")

    # Each published package carries its own copy of the licences.
    packages = [ROOT / "bindings/python", *sorted(p.parent for p in ROOT.glob("crates/*/Cargo.toml"))]
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        for package in packages:
            copy = package / name
            if not copy.exists() or copy.read_bytes() != (ROOT / name).read_bytes():
                problems.append(f"{copy.relative_to(ROOT)} is missing or differs from {name}")

    problems += readme_problems("--run-examples" in sys.argv)

    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if not problems:
        print(f"consistent: version {version}, licence copies match, READMEs checked")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
