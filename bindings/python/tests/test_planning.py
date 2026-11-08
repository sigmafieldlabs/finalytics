"""Plans: a goal, the money paid in and out, the mix and the inflation, from plain inputs."""

import datetime

import pandas as pd
import pytest

import finalytics as fl


@pytest.fixture(scope="module")
def assets() -> fl.Assets:
    return fl.Assets(fl.datasets.asset_class_prices())


def house(assets, **changes):
    options = dict(
        start_value=60_000,
        flows=[fl.Saving(amount=2_500, every="month"), fl.Spending(name="Car", amount=25_000, on="2028-03")],
        weights={"US Equity": 0.6, "Bonds": 0.4},
        inflation=0.025,
        fee=0.0025,
        simulations=1_000,
    )
    options.update(changes)
    return assets.plan(fl.Target(amount=150_000, by="2030-06"), **options)


def test_a_target_has_a_chance_a_status_and_changes(assets):
    plan = house(assets, name="House deposit", currency="USD")
    r = plan.results
    assert 0 < r.outcome.chance < 1
    assert r.start == "2026-11" and r.end == "2030-06"
    assert r.status in ("room_to_spare", "on_track", "nearly_on_track", "off_track")
    assert r.changes is not None and r.changes.to_reach == (r.outcome.chance < 0.8)
    assert len(r.sample) == 100
    assert [f.name for f in r.flows] == ["Monthly saving", "Car"]
    assert r.flows[1].count == 1 and r.flows[1].paid_in_full is not None
    assert plan.report.context.goal.name == "House deposit"
    text = str(plan)
    assert text.startswith("━") and "FINALYTICS  ·  PLANNING" in text and "House deposit" in text
    assert all(len(line) <= 80 for line in text.splitlines())
    assert plan.report.to_html().startswith("<!doctype html>")
    assert "PlanResults" in str(r)
    names = [t.name for t in fl.reports.tables(plan.report)]
    assert names[:3] == ["outcome", "flows", "value_over_time"] and "mixes" in names


def test_the_same_seed_gives_the_same_plan(assets):
    a, b = house(assets, seed=7), house(assets, seed=7)
    assert a.results == b.results
    assert house(assets, seed=8).results != a.results


def test_payments_say_when_the_money_runs_short(assets):
    plan = assets.plan(
        fl.Payments(amount=4_000, start="2046-11", end="2066-10"),
        start_value=250_000,
        flows=[fl.Saving(amount=1_500, every="month")],
        weights=("Balanced 60/40", {"US Equity": 0.6, "Bonds": 0.4}),
        alternatives={"Cautious 30/70": {"US Equity": 0.3, "Bonds": 0.7}},
        inflation=0.025,
        chance=(0.85, 0.9),
        simulations=500,
    )
    r = plan.results
    assert r.chance_wanted == 0.85 and r.spare_from == 0.9
    assert r.payments is not None and 0 < r.payments.paid <= 1
    assert [a.name for a in r.alternatives] == ["Cautious 30/70"]
    assert plan.report.context.mix_name == "Balanced 60/40"


def steady_index() -> pd.DataFrame:
    """A price index rising 2.5% a year, on the 28th of each month from January 1990 to September
    2026, without a level for October 2025."""
    months = [k for k in range(441) if (1990 + k // 12, k % 12 + 1) != (2025, 10)]
    return pd.DataFrame(
        {"Prices": [100 * 1.025 ** (k / 12) for k in months]},
        index=pd.DatetimeIndex([datetime.date(1990 + k // 12, k % 12 + 1, 28) for k in months], name="date"),
    )


def test_a_price_index_and_a_glide_path(assets):
    plan = house(
        assets,
        inflation=steady_index(),
        weights=fl.GlidePath(points={"2026-11": {"US Equity": 0.8, "Bonds": 0.2}, "2030-01": {"Bonds": 1.0}}),
        model="normal",
    )
    basis = plan.report.context.inflation
    assert isinstance(basis, fl.types.InflationBasisIndex) and basis.name == "Prices"
    assert abs(basis.average - 0.025) < 1e-12
    # The index ends a month before the assets, so the history does too.
    assert plan.report.context.history_end == "2026-09"


def test_a_projection_has_no_chance(assets):
    plan = assets.plan(
        end=datetime.date(2040, 12, 1),
        start_value=10_000,
        flows=[fl.Saving(amount=500, every="month")],
        weights={"US Equity": 1.0},
        inflation=0.02,
        model=fl.Bootstrap(block=6),
        simulations=200,
    )
    assert plan.results.outcome.chance is None and plan.results.status is None
    assert plan.results.end == "2041-01"


def test_problems_are_reported_together(assets):
    with pytest.raises(fl.ValidationError) as caught:
        assets.plan(
            fl.Target(amount=-1, by="2030-06"),
            weights={"Nope": 1.0},
            inflation=0.02,
            fee=2,
        )
    paths = {issue.path for issue in caught.value.issues}
    assert {"goal.amount", "fee"} <= paths and any(p.startswith("weights") for p in paths)
    with pytest.raises(fl.ValidationError):
        assets.plan(fl.Target(amount=1, by="2030-06"), weights={"US Equity": 1.0}, inflation=0.02, model="monte")


GOALS = """\
goal,type,amount,by,every,start,end,name,weights,chance,start_value,inflation,simulations
House,target,150000,2030-06,,,,,,0.8,60000,0.025,1000
House,saving,2500,,month,,,,,,,,
House,mix,,,,,,Balanced,US Equity=0.6;Bonds=0.4,,,,
"""


def test_goals_kept_with_a_portfolio_plan_as_they_are(assets):
    files = {name: fl.datasets.file(f"portfolio/{name}.csv") for name in ("transactions", "instruments", "accounts", "prices", "fx")}
    portfolio = fl.Portfolio.from_csv(**files, goals=GOALS)
    [goal] = portfolio.goals
    assert goal.name == "House" and goal.currency == portfolio.data.base_currency
    stored = assets.plan(goal)
    built = assets.plan(
        fl.Target(name="House", amount=150_000, by="2030-06"),
        start_value=60_000,
        flows=[fl.Saving(amount=2_500, every="month")],
        weights=("Balanced", {"US Equity": 0.6, "Bonds": 0.4}),
        inflation=0.025,
        simulations=1_000,
        currency=portfolio.data.base_currency,
        name="House",
    )
    assert stored.results == built.results
    # Saved with the portfolio, and planned the same when read back.
    read = fl.Portfolio.read(portfolio.to_json())
    assert read.goals == portfolio.goals
    assert fl.datasets.portfolio().with_goals(read.goals).goals == read.goals
    with pytest.raises(fl.ValidationError) as caught:
        assets.plan(goal, seed=3)
    assert caught.value.issues[0].path == "seed"
