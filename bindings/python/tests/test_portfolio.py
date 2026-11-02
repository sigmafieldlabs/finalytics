import datetime
import decimal
import json

import numpy as np
import pytest

import finalytics as fl
from finalytics import types as t

from conftest import ROOT, SCENARIO


def test_csv_import_builds_the_portfolio(portfolio):
    assert portfolio.base_currency == "USD"
    assert [a.id for a in portfolio.accounts] == ["isa", "main"]
    assert [i.id for i in portfolio.instruments] == ["SPY", "SAP", "BOND"]
    assert portfolio.instruments[1].classifications == {"asset_class": "Equity", "region": "Europe"}
    buy = portfolio.transactions[1]
    assert isinstance(buy, t.Transaction) and isinstance(buy.kind, t.Buy)
    assert buy.kind.quantity == decimal.Decimal("100") and buy.trade_date == datetime.date(2026, 9, 1)
    dividend = portfolio.transactions[7].kind
    assert isinstance(dividend, t.Dividend) and dividend.withholding_tax == decimal.Decimal("27")
    assert portfolio.validate() == []


def test_csv_problems_are_reported_together():
    with pytest.raises(fl.ValidationError) as caught:
        fl.PortfolioData.from_csv(
            instruments="id,kind,currency\nAAPL,equity,usd\n",
            transactions="id,account,trade_date,type,instrument,quantity,price\nt1,main,2026-01-02,buy,AAPL,ten,1\n",
            base_currency="USD",
        )
    paths = [i.path for i in caught.value.issues]
    assert paths == ["instruments.line 2 (currency)", "transactions.line 2 (quantity)"]
    assert all(i.severity == "error" for i in caught.value.issues)


def test_portfolios_round_trip_through_dataclasses_and_json(portfolio):
    again = fl.PortfolioData.from_json(portfolio.to_json())
    assert again == portfolio
    built = fl.PortfolioData(
        base_currency="USD",
        accounts=[t.Account(id="main")],
        instruments=[t.Instrument(id="AAPL", kind="equity", currency="USD")],
        transactions=[
            t.Transaction(
                id="t1",
                account="main",
                trade_date=datetime.date(2026, 1, 2),
                kind=t.Deposit(amount=t.Money(amount=decimal.Decimal("1000"), currency="USD")),
            ),
            t.Transaction(
                id="t2",
                account="main",
                trade_date=datetime.date(2026, 1, 5),
                kind=t.Buy(instrument="AAPL", quantity=decimal.Decimal("5"), price=decimal.Decimal("150")),
            ),
        ],
    )
    data = json.loads(built.to_json())
    assert data["transactions"][1] == {
        "id": "t2",
        "account": "main",
        "trade_date": "2026-01-05",
        "type": "buy",
        "instrument": "AAPL",
        "quantity": "5",
        "price": "150",
        "fees": "0",
    }
    books = fl.Portfolio(built)
    assert books.holdings().cash[0].amount == decimal.Decimal("250")


def test_documents_hold_the_portfolio_and_market(portfolio, market):
    text = fl.write_document(portfolio, market)
    document = fl.read_document(text)
    assert document.portfolio == portfolio
    assert document.market.prices == market.prices
    with pytest.raises(fl.ValidationError) as caught:
        fl.read_document('{"format": "finalytics.portfolio", "format_version": 2, "portfolio": {}}')
    assert caught.value.issues[0].path == "format_version"


def test_market_data_lookups(market):
    assert market.pivot == "USD" and market.max_carry_days == 5
    assert len(market.price_dates) == 44
    found = market.price("SPY", "2026-09-05")  # a Saturday: Friday's price is carried
    assert found.carried and found.date == datetime.date(2026, 9, 4)
    assert market.price("SPY", "2026-12-31") is None
    inverse = market.fx_rate("USD", "EUR", "2026-09-01")
    assert inverse.value == decimal.Decimal(1) / decimal.Decimal("1.10000")
    series = fl.MarketData(prices={"X": {"2026-01-02": 10, "2026-01-05": 10.5}})
    assert series.prices["X"].values == [decimal.Decimal("10"), decimal.Decimal("10.5")]
    with pytest.raises(ValueError):
        fl.MarketData(prices={"X": (["2026-01-02"], [float("nan")])})


def test_holdings_and_valuation(books):
    assert books.base_currency == "USD"
    holdings = books.holdings("2026-09-30")
    spy = [p for p in holdings.positions if p.instrument == "SPY"]
    assert sorted((p.account, p.quantity) for p in spy) == [("isa", 30), ("main", 100)]
    valuation = books.valuation("2026-10-30")
    total = sum(p.market_value_base for p in valuation.positions) + sum(c.value_base for c in valuation.cash)
    assert total == valuation.total_base
    gains = books.realised_gains()
    assert [g.instrument for g in gains] == ["SPY"] and gains[0].quantity == 40
    events = books.events()
    assert len(events) == 11 and isinstance(events[0].effects[0], t.CashChange)
    weights = books.weights("2026-10-30")
    assert abs(sum(weights.values()) - 1.0) < 1e-12
    assert "SAP" in weights and fl.cash("EUR") == "EUR cash" in weights


def test_lot_preview_and_rebalancing(books, portfolio, market):
    lots = books.lots_for_sale("2026-10-30", "main", "SPY", 10)
    assert [(l.lot, l.quantity) for l in lots] == [("t2", 10)]
    proposal = books.rebalance({"SPY": 0.4, "BOND": 0.4, "SAP": 0.15}, accounts=["main"], lot_method="highest_cost")
    [plan] = proposal.accounts
    assert plan.date == datetime.date(2026, 10, 30)
    assert {trade.instrument for trade in plan.trades} == {"SPY", "BOND", "SAP"}
    after = fl.PortfolioData(
        base_currency=portfolio.base_currency,
        accounts=portfolio.accounts,
        instruments=portfolio.instruments,
        transactions=[*portfolio.transactions, *fl.opt.transactions(plan)],
    )
    rebuilt = fl.Portfolio(after, market)
    assert rebuilt.valuation().total_base == books.with_rebalance(proposal).valuation().total_base
    main = [p for p in rebuilt.valuation(plan.date).positions if p.account == "main"]
    value = {p.instrument: p.market_value_base for p in main}
    assert abs(float(value["SPY"] / plan.value_base) - 0.4) < 1e-6


def test_rebalancing_with_fees_and_tax(books, portfolio, market):
    fees = fl.opt.Fees(rate=decimal.Decimal("0.001"), minimum=decimal.Decimal("5"))
    tax = fl.opt.TaxRates(short_term=decimal.Decimal("0.4"), long_term=decimal.Decimal("0.2"))
    [plan] = books.rebalance({"SPY": 0.2, "BOND": 0.5, "SAP": 0.15}, accounts=["main"], fees=fees, tax=tax).accounts
    assert all(t.fees_local >= 5 for t in plan.trades)
    sales = [t for t in plan.trades if t.side == "sell"]
    assert sales and all(t.tax_base is not None for t in sales)
    assert plan.tax_base == sum(t.tax_base for t in sales)
    after = fl.PortfolioData(
        base_currency=portfolio.base_currency,
        accounts=portfolio.accounts,
        instruments=portfolio.instruments,
        transactions=[*portfolio.transactions, *fl.opt.transactions(plan)],
    )
    rebuilt = fl.Portfolio(after, market)
    gains = sum(g.gain_base for g in rebuilt.realised_gains() if g.transaction.startswith("rebalance-"))
    assert gains == plan.realised_gain_base


def test_rebalancing_several_accounts(books):
    accounts = sorted({a.id for a in books.data.accounts})
    plan = books.rebalance({"SPY": 0.3, "BOND": 0.4}, accounts=accounts)
    assert [p.account for p in plan.accounts] == [a for a in accounts if any(p.account == a for p in plan.accounts)]
    after = dict(plan.weights_after)
    assert abs(after["SPY"] - 0.3) < 1e-6
    assert abs(after["BOND"] - 0.4) < 1e-6
