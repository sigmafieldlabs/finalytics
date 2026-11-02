import json

import pytest

import finalytics as fl

from conftest import FIXTURES, SCENARIO

HOUSEHOLDS = json.loads((FIXTURES / "finalytics/tests/fixtures/households.json").read_text())


@pytest.fixture(scope="module")
def household_books(market) -> fl.Portfolio:
    portfolio = fl.PortfolioData.from_csv(
        instruments=SCENARIO / "instruments.csv",
        transactions=SCENARIO / "transactions.csv",
        accounts=SCENARIO / "accounts.csv",
        base_currency="USD",
    )
    return fl.Portfolio(portfolio, market)


def compare(found, expected):
    assert found.accounts == expected["accounts"]
    for name in ("value_start", "value_end", "flows", "time_weighted", "contribution"):
        assert getattr(found, name) == pytest.approx(expected[name], rel=1e-12, abs=1e-12), name
    assert found.money_weighted == pytest.approx(expected["money_weighted"], rel=1e-10)


def test_accounts_from_csv_keep_their_classifications(household_books):
    accounts = household_books.data.accounts
    assert [a.id for a in accounts] == ["main", "isa"]
    assert accounts[1].classifications == {"owner": "Alice", "wrapper": "ISA"}
    assert accounts[0].name == "Brokerage account"


def test_household_matches_the_reference(household_books):
    found = household_books.household(HOUSEHOLDS["dates"], group_by=["owner", "wrapper"])
    compare(found.household, HOUSEHOLDS["household"])
    for account, expected in zip(found.accounts, HOUSEHOLDS["accounts"], strict=True):
        assert account.name == expected["name"]
        compare(account, expected)
    for scheme in found.groups:
        for group, expected in zip(scheme.groups, HOUSEHOLDS["groups"][scheme.scheme], strict=True):
            assert group.name == expected["name"]
            compare(group, expected)
    assert sum(a.contribution for a in found.accounts) == pytest.approx(found.household.time_weighted, abs=1e-15)


def test_account_series_add_up_to_the_value_series(household_books):
    series = household_books.account_series()
    whole = household_books.value_series()
    totals = [sum(line.values[t] for line in series.lines) for t in range(len(series.dates))]
    assert totals == whole.values


def test_unclassified_accounts_have_a_group(books):
    found = books.household(group_by=["owner"])
    assert [g.name for g in found.groups[0].groups] == ["unclassified"]
    assert found.groups[0].groups[0].accounts == ["isa", "main"]
