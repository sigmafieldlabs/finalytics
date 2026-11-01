#!/usr/bin/env python3
"""Check that version numbers and licence copies agree across the repository."""

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    problems = []
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    version = workspace["workspace"]["package"]["version"]

    package = json.loads((ROOT / "bindings/wasm/package.json").read_text())
    if package["version"] != version:
        problems.append(f"bindings/wasm/package.json has version {package['version']}, Cargo.toml has {version}")

    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        if (ROOT / name).read_bytes() != (ROOT / "bindings/python" / name).read_bytes():
            problems.append(f"bindings/python/{name} differs from {name}")

    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if not problems:
        print(f"consistent: version {version}, licence copies match")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
