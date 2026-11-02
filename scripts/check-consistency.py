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

    # Each published package carries its own copy of the licences.
    packages = [ROOT / "bindings/python", *sorted(p.parent for p in ROOT.glob("crates/*/Cargo.toml"))]
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        for package in packages:
            copy = package / name
            if not copy.exists() or copy.read_bytes() != (ROOT / name).read_bytes():
                problems.append(f"{copy.relative_to(ROOT)} is missing or differs from {name}")

    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if not problems:
        print(f"consistent: version {version}, licence copies match")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
