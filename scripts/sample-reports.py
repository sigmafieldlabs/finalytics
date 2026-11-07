"""Builds the sample reports that the READMEs show and link to: the seven reports the READMEs'
programs build from the bundled datasets, as HTML pages, with an index page and, with
--screenshots, the top of each page in light and dark for the READMEs' images.

    python scripts/sample-reports.py site --screenshots

It needs the finalytics Python package, and Playwright with Chromium for the screenshots. The
Pages workflow (.github/workflows/pages.yml) runs it on each release and publishes the folder.
"""

from __future__ import annotations

import html
import pathlib
import sys

# The reports, in the order the READMEs show them: (file name, title, what it shows).
REPORTS = [
    ("comparison", "Comparison report", "Six optimisation methods tested walk-forward on 30 years of six asset classes."),
    ("backtest", "Backtest report", "The maximum Sharpe ratio held from November 2016 against a 60/40 benchmark."),
    ("optimisation", "Optimisation report", "The maximum Sharpe ratio's weights, their risk and the efficient frontier."),
    ("portfolio", "Portfolio report", "A household's two accounts: holdings, performance, risk, attribution and scenarios."),
    ("rebalance", "Rebalance report", "The trades that move both accounts to the least risky mix of their funds, tax-aware."),
    ("planning", "Planning report", "A retirement income's chance of lasting 30 years, what would change it, and when markets do worse."),
    ("screening", "Screening report", "Five funds and their benchmark screened by return, volatility and asset class, with the spread of every measure."),
]

# The size of the screenshots: the top of each page.
WIDTH, HEIGHT = 1200, 760


def build() -> dict[str, object]:
    """The seven results, built as the READMEs' Python programs build them."""
    import finalytics as fl

    assets = fl.Assets(prices=fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    comparison = assets.compare(
        methods=[
            "equal_weight",
            "minimum_variance",
            "maximum_sharpe",
            "maximum_diversification",
            "risk_parity",
            "hierarchical_risk_parity",
        ],
        max_weight=0.40,
        rebalance="quarterly",
        validation=fl.WalkForward(years=5),
        trading_cost_bps=10,
    )
    backtest = assets.backtest(
        method="maximum_sharpe",
        max_weight=0.40,
        benchmark={"US Equity": 0.60, "Bonds": 0.40},
        rebalance="quarterly",
        validation=fl.Holdout(start="2016-11-01"),
    )
    optimisation = assets.optimise(method="maximum_sharpe", max_weight=0.40, frontier=True)

    portfolio = fl.datasets.portfolio()
    # The README's report, with the dataset's attribution and scenarios.
    report = portfolio.report(fl.datasets.report_options())
    target = portfolio.assets().optimise(method="minimum_variance", bounds=(0.05, 0.40))
    rebalancing = portfolio.rebalance(target, whole_units=True, fee_bps=5, minimum_fee=1, tax=(0.37, 0.20))
    plan = fl.Assets(prices=fl.datasets.asset_class_prices()).plan(
        fl.Payments(amount=4_000, start="2046-11", end="2076-10"),
        name="Retirement income",
        chance=0.85,
        start_value=250_000,
        flows=[fl.Saving(amount=1_500, every="month")],
        weights=("Balanced 60/40", {"US Equity": 0.60, "Bonds": 0.40}),
        alternatives={
            "Cautious 30/70": {"US Equity": 0.30, "Bonds": 0.70},
            "Growth 80/20": {"US Equity": 0.80, "Bonds": 0.20},
        },
        inflation=0.025,
        fee=0.0025,
        currency="USD",
    )
    files = fl.datasets.file
    screen = fl.Universe(
        files("portfolio/prices.csv"),
        fields=files("portfolio/instruments.csv"),
        benchmark="BENCH",
        risk_free_rate=0.04,
    ).screen(
        rules=[
            ("return_1y", ">", 0.0),
            ("volatility", "<", 0.20),
            ("class.asset_class", "in", ["Equity", "Real assets"]),
        ],
        sort=("sharpe_ratio", "descending"),
    )
    return {
        "comparison": comparison,
        "backtest": backtest,
        "optimisation": optimisation,
        "portfolio": report,
        "rebalance": rebalancing,
        "planning": plan,
        "screening": screen,
    }


def index_page(version: str) -> str:
    """A page that lists the reports, each with its picture."""
    cards = "".join(
        f'<a class="card" href="reports/{name}.html"><picture>'
        f'<source media="(prefers-color-scheme: dark)" srcset="images/{name}-dark.png">'
        f'<img src="images/{name}-light.png" alt="{html.escape(title)}" width="{WIDTH}" height="{HEIGHT}" '
        f'loading="lazy"></picture><span class="t">{html.escape(title)}</span>'
        f'<span class="d">{html.escape(text)}</span></a>'
        for name, title, text in REPORTS
    )
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Finalytics sample reports</title>
<style>
:root{{--bg:#f9f9f7;--surface:#fcfcfb;--ink:#0b0b0b;--ink2:#52514e;--ring:rgba(11,11,11,.10);--link:#2a78d6}}
@media (prefers-color-scheme:dark){{:root{{--bg:#0d0d0d;--surface:#1a1a19;--ink:#f0efec;--ink2:#c3c2b7;--ring:rgba(255,255,255,.10);--link:#3987e5;color-scheme:dark}}}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--bg);color:var(--ink);font:15px/1.45 system-ui,-apple-system,"Segoe UI",sans-serif}}
main{{max-width:1100px;margin:0 auto;padding:32px max(16px,env(safe-area-inset-left,0px)) 64px}}
h1{{margin:0;font-size:28px;font-weight:600}}
p{{color:var(--ink2);max-width:70ch}}
a{{color:var(--link)}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:16px;margin-top:24px}}
.card{{display:flex;flex-direction:column;gap:4px;background:var(--surface);border:1px solid var(--ring);border-radius:12px;
padding:12px;text-decoration:none;color:var(--ink)}}
.card:hover,.card:focus-visible{{border-color:var(--link)}}
.card img{{width:100%;height:auto;border-radius:8px;border:1px solid var(--ring)}}
.t{{font-weight:600;margin-top:6px}}.d{{color:var(--ink2);font-size:14px}}
footer{{margin-top:40px;color:var(--ink2);font-size:13px}}
</style></head>
<body><main>
<h1>Finalytics sample reports</h1>
<p>Each report is one self-contained page, built by Finalytics {html.escape(version)} from the datasets that come
with it, by the programs in the <a href="https://github.com/sigmafieldlabs/finalytics#readme">README</a>.</p>
<div class="grid">{cards}</div>
<footer>Finalytics provides analysis tools only. It is not investment advice.</footer>
</main></body></html>
"""


def screenshots(out: pathlib.Path) -> None:
    """The top of each report, in light and dark."""
    from playwright.sync_api import sync_playwright

    (out / "images").mkdir(exist_ok=True)
    with sync_playwright() as p:
        browser = p.chromium.launch()
        for name, _, _ in REPORTS:
            for scheme in ("light", "dark"):
                page = browser.new_page(viewport={"width": WIDTH, "height": HEIGHT}, color_scheme=scheme)
                page.goto((out / "reports" / f"{name}.html").resolve().as_uri())
                page.screenshot(path=str(out / "images" / f"{name}-{scheme}.png"))
                page.close()
        browser.close()


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__)
        return 2
    out = pathlib.Path(args[0])
    (out / "reports").mkdir(parents=True, exist_ok=True)
    results = build()
    for name, result in results.items():
        (out / "reports" / f"{name}.html").write_text(result.to_html(), encoding="utf-8")
    version = results["comparison"].report.context.version
    (out / "index.html").write_text(index_page(version), encoding="utf-8")
    if "--screenshots" in sys.argv:
        screenshots(out)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
