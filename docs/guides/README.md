# The guides

Task-by-task guides to Finalytics, with examples in Python, JavaScript and Rust. They are
published at [sigmafield.app/libraries/finalytics](https://sigmafield.app/libraries/finalytics),
where the Python and JavaScript examples run in the page. This folder is their source, and every
example in it runs in CI on each change, so the guides and the library cannot drift apart.

## Layout

| Path | What it holds |
| --- | --- |
| `contents.json` | The sidebar: its groups in order, each with its folder and its pages. |
| `<group>/<page>.md` | A guide. Its file name is its address: `optimise/optimise-a-portfolio.md` is published as `guides/optimise-a-portfolio`, so no two pages share a name. |
| `../examples/<page>/<example>.py`, `.js`, `.rs` | The page's examples: one program each, in each of its languages. |
| `../examples/<page>/<example>.out` | What the example prints, recorded by `scripts/guides.py --record`. |
| `../examples/Cargo.toml` | The Rust examples as one package, `finalytics-guides`, which is not published. |

## A page

A page starts with front matter, then its title in the imperative, named for the task:

```markdown
---
description: Choose weights for a set of assets with any of the 15 methods, and read what the result holds.
learn: portfolio/1.4
workstation: portfolio/optimise
---

# Optimise a portfolio
```

- `description` (required): the lead under the title, one or two sentences on what the reader
  will be able to do.
- `learn`: the lesson or module that teaches the method, such as `portfolio/1.4`, for the "Learn
  the method" card.
- `workstation`: the workstation area that does the same task without code, such as
  `portfolio/optimise`, for the "Use it without code" card.
- `numbers`: numbers the prose states that are not results, such as `[3.10]` for a Python
  version (see below).

Sections are `##` headings, which make the page's "On this page" list. A page goes from the task
in one example to its options and edge cases, with an example in most sections.

**Examples.** An example appears where a paragraph holds only a link to its Python file:

```markdown
[optimise.py](../../examples/optimise-a-portfolio/optimise.py)
```

The site shows it there in each of its languages, with its output; on GitHub, the link opens the
file. An example belongs to one page, and lives in the folder named after it.

**Callouts.** Conventions, limits and pitfalls go where readers meet them, as GitHub's alerts:
`> [!NOTE]`, `> [!TIP]` or `> [!WARNING]`, followed by the text on the next lines of the quote.

## An example

- **One whole program.** It imports what it uses and runs on its own, so that a reader can start
  at any example. The same code runs in the page, in a terminal and in a notebook.
- **The same in every language.** Each language's program prints the same output, character for
  character, and CI fails if they differ. Print what the engine lays out (a report, a table of
  results), or round numbers the same way in each language. An example leaves out a language
  only when the feature is not in it, such as the pandas helpers, which are Python's.
- **The data the packages ship.** `datasets` in each language: the household portfolio, the six
  asset classes and the universe of 200 companies for screening. An example may also read the
  household's files by name (`transactions.csv`, `prices.csv`, `fx.csv`, `instruments.csv` and
  `accounts.csv`): the folder it runs in holds them, as a reader's working folder would.
- **`show()` is welcome.** In CI it opens nothing; on the site, the page appears with the output.
- **80 columns.** The code fits the page on a laptop, and scrolls sideways on a phone.
- **Rust** examples are programs with `fn main() -> Result<()>`, listed in `Cargo.toml` by the
  recording script.

## Numbers in the prose

Every decimal, percentage or amount of money in a page's prose is one its examples print, as
printed or rounded, or one they are given: `max_weight=0.40` lets the prose say 40%. Whole
numbers, version numbers and numbers in code spans are not checked. A number that is neither,
such as a Python version, is listed under `numbers` in the front matter. The rule keeps every
figure a reader sees in step with the library that produced it.

## Running and recording

```sh
python3 scripts/guides.py --record   # after a change: run each example and record its output
python3 scripts/guides.py            # as CI does: run every example and compare
```

The script runs Python with the package installed in the interpreter that runs it (`maturin
develop` in `bindings/python`), JavaScript on the npm package built by `bindings/wasm/build.sh`,
and Rust with cargo. `--page optimise-a-portfolio` runs one page's examples; `--languages
python,javascript` some languages; `--python PATH` another interpreter, such as a Pyodide
environment's (`pyodide venv`), to check the examples in Python running on WebAssembly.
