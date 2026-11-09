"""Portfolios: their records, the market data that values them, and the books kept from them."""

from __future__ import annotations

import dataclasses
import datetime
import decimal
import json
import math
import os
import pathlib
from collections.abc import Iterable, Mapping, Sequence
from typing import TYPE_CHECKING, Any, NamedTuple

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._arrays import DateArray, DateLike
from finalytics._convert import dumps, from_json, loads, to_json
from finalytics._errors import call, invalid

if TYPE_CHECKING:
    from finalytics.assets import Assets

TextOrPath = str | os.PathLike[str]
"""CSV or JSON given as text, or as the path of a file holding it."""


def _text(source: TextOrPath) -> str:
    """The text of `source`: a path is read; a string is a path if it names a file, and is
    otherwise the content itself."""
    if isinstance(source, os.PathLike):
        return pathlib.Path(source).read_text(encoding="utf-8")
    if "\n" not in source and pathlib.Path(source).is_file():
        return pathlib.Path(source).read_text(encoding="utf-8")
    if "\n" not in source and source.lower().endswith((".csv", ".json", ".txt")):
        raise FileNotFoundError(source)
    return source


def cash(currency: str) -> _types.HoldingKey:
    """The holding of cash in one currency, added up across accounts: ``cash("USD")`` is
    ``"USD cash"``. An instrument's holding is its identifier."""
    return f"{currency} cash"


class PortfolioData(_types.PortfolioData):
    """A portfolio's records: its base currency, accounts, instruments and transactions. With
    :class:`MarketData`, they make a :class:`Portfolio`.

    Build them from dataclasses, from JSON, or from CSV files.
    """

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> PortfolioData:
        """Records from decoded JSON in the portfolio format."""
        return from_json(cls, data)

    @classmethod
    def from_json(cls, text: str) -> PortfolioData:
        """Records from JSON text in the portfolio format (the ``portfolio`` member of a
        portfolio document)."""
        return loads(cls, text)

    @classmethod
    def from_csv(
        cls,
        *,
        transactions: TextOrPath,
        instruments: TextOrPath | None = None,
        accounts: TextOrPath | None = None,
        base_currency: str | None = None,
        delimiter: str = ",",
    ) -> PortfolioData:
        """Records from CSV files; see :meth:`Portfolio.from_csv` for what each holds and what
        is assumed without it.

        Every problem is reported at once, by line and column, in a
        :class:`~finalytics.ValidationError`.
        """
        text = call(
            _finalytics.read_portfolio_csv,
            _text(transactions),
            delimiter,
            None if instruments is None else _text(instruments),
            None if accounts is None else _text(accounts),
            base_currency,
        )
        return cls.from_json(text)

    def to_json(self) -> str:
        """The records as JSON text."""
        return dumps(self)

    def validate(self) -> list[_types.Issue]:
        """Every structural problem in the records, without applying the transactions.

        Problems found only by applying the transactions in order, such as selling more than is
        held, are reported when the :class:`Portfolio` is built.
        """
        report = loads(_types.ValidationReport, call(_finalytics.validate_portfolio, self.to_json()))
        return report.issues


class Observed(NamedTuple):
    """A price or rate found for a date."""

    value: decimal.Decimal
    """The value."""
    date: datetime.date
    """The date it was observed, earlier than the date asked for if it was carried forward."""
    carried: bool
    """Whether it was carried forward from an earlier date."""


def _decimal_text(value: Any) -> str:
    if isinstance(value, (decimal.Decimal, int, str)):
        return str(value)
    number = float(value)
    if not math.isfinite(number):
        raise invalid(("prices", f"prices and rates must be finite numbers, not {number}"))
    return repr(number)


def _series(series: Any) -> tuple[Any, list[str]]:
    """Dates and decimal values from a TimeSeries, a ``(dates, values)`` pair, a mapping of dates
    to values, or a pandas Series indexed by date."""
    if isinstance(series, _types.TimeSeries):
        return series.dates, [str(v) for v in series.values]
    if isinstance(series, tuple) and len(series) == 2:
        dates, values = series
    elif isinstance(series, Mapping):
        dates, values = list(series.keys()), list(series.values())
    elif hasattr(series, "index") and hasattr(series, "to_numpy"):
        dates, values = series.index, series.to_numpy()
    else:
        raise TypeError("a series must be a TimeSeries, a (dates, values) pair, a mapping or a pandas Series")
    return [_arrays.date(d) for d in dates], [_decimal_text(v) for v in values]


class MarketData:
    """Prices and FX rates for valuing a portfolio.

    Prices are in each instrument's currency; an FX rate is the number of units of `quote` per
    unit of `base`. A rate can be found directly, inverted, or crossed through the pivot
    currency. A missing value may be carried forward from an earlier date for at most
    `max_carry_days` days, and carried values are reported as warnings.

    Each method that adds data returns a new :class:`MarketData`.
    """

    def __init__(
        self,
        prices: Mapping[str, Any] | None = None,
        fx: Iterable[tuple[str, str, Any]] | None = None,
        *,
        pivot: str = "USD",
        max_carry_days: int = 5,
    ):
        native = _finalytics.Market()
        native = call(native.with_pivot, pivot)
        native = native.with_max_carry_days(max_carry_days)
        self._native = native
        for instrument, series in (prices or {}).items():
            self._native = self._add_prices(instrument, series)
        for base, quote, series in fx or ():
            self._native = self._add_fx(base, quote, series)

    @classmethod
    def _wrap(cls, native: _finalytics.Market) -> MarketData:
        market = cls.__new__(cls)
        market._native = native
        return market

    @classmethod
    def from_csv(
        cls,
        *,
        prices: TextOrPath | None = None,
        fx: TextOrPath | None = None,
        pivot: str = "USD",
        max_carry_days: int = 5,
        delimiter: str = ",",
    ) -> MarketData:
        """Market data from CSV files of prices and FX rates, in long (``date,instrument,price``
        or ``date,base,quote,rate``) or wide (one column per instrument or pair) layout."""
        market = cls(pivot=pivot, max_carry_days=max_carry_days)
        if prices is not None:
            for instrument, series in json.loads(call(_finalytics.read_prices_csv, _text(prices), delimiter)).items():
                market = market.with_prices(instrument, (series["dates"], series["values"]))
        if fx is not None:
            for entry in json.loads(call(_finalytics.read_fx_csv, _text(fx), delimiter)):
                rates = entry["rates"]
                market = market.with_fx(entry["base"], entry["quote"], (rates["dates"], rates["values"]))
        return market

    def _add_prices(self, instrument: str, series: Any) -> _finalytics.Market:
        dates, values = _series(series)
        return call(self._native.with_prices, instrument, _arrays.dates_to_buffer(dates), json.dumps(values))

    def _add_fx(self, base: str, quote: str, series: Any) -> _finalytics.Market:
        dates, values = _series(series)
        return call(self._native.with_fx, base, quote, _arrays.dates_to_buffer(dates), json.dumps(values))

    def with_prices(self, instrument: str, series: Any) -> MarketData:
        """These market data with prices for `instrument`: a :class:`~finalytics.types.TimeSeries`,
        a ``(dates, values)`` pair, a mapping of dates to values, or a pandas Series."""
        return MarketData._wrap(self._add_prices(instrument, series))

    def with_fx(self, base: str, quote: str, series: Any) -> MarketData:
        """These market data with rates for `base` priced in `quote`."""
        return MarketData._wrap(self._add_fx(base, quote, series))

    @property
    def pivot(self) -> str:
        """The currency through which rates are crossed."""
        return self._native.pivot()

    @property
    def max_carry_days(self) -> int:
        """How many days a price or rate may be carried forward."""
        return self._native.max_carry_days()

    @property
    def prices(self) -> dict[str, _types.TimeSeries]:
        """The prices, by instrument."""
        return from_json(dict[str, _types.TimeSeries], json.loads(self._native.prices()))

    @property
    def fx(self) -> list[_types.DocumentFxRates]:
        """The FX rates, by currency pair."""
        return from_json(list[_types.DocumentFxRates], json.loads(self._native.fx()))

    @property
    def price_dates(self) -> DateArray:
        """Every date with a price for any instrument, in order."""
        return _arrays.dates_from_buffer(self._native.price_dates())

    def price(self, instrument: str, date: DateLike) -> Observed | None:
        """The price of `instrument` on `date`, or None if there is none within the carry limit."""
        return _observed(call(self._native.price, instrument, _arrays.date(date)))

    def fx_rate(self, base: str, quote: str, date: DateLike) -> Observed | None:
        """Units of `quote` per unit of `base` on `date`, or None if no rate is found."""
        return _observed(call(self._native.fx_rate, base, quote, _arrays.date(date)))

    def __repr__(self) -> str:
        return f"MarketData({len(self.prices)} instruments, {len(self.fx)} currency pairs, pivot={self.pivot})"


def _observed(text: str | None) -> Observed | None:
    if text is None:
        return None
    found = json.loads(text)
    return Observed(decimal.Decimal(found["value"]), datetime.date.fromisoformat(found["date"]), found["carried"])


class Document(NamedTuple):
    """A portfolio document: a portfolio's records and, optionally, its market data."""

    portfolio: PortfolioData
    market: MarketData | None


def read_document(source: TextOrPath) -> Document:
    """Reads a portfolio document (``"format": "finalytics.portfolio"``), reporting where a
    malformed one goes wrong. :meth:`Portfolio.read` reads one into a portfolio."""
    portfolio, market = call(_finalytics.read_document, _text(source))
    return Document(PortfolioData.from_json(portfolio), None if market is None else MarketData._wrap(market))


def write_document(portfolio: PortfolioData, market: MarketData | None = None) -> str:
    """A portfolio document holding `portfolio` and, optionally, `market`, as JSON text."""
    return call(_finalytics.write_document, portfolio.to_json(), None if market is None else market._native)


@dataclasses.dataclass(frozen=True, kw_only=True)
class Holdings:
    """Open positions, tax lots and cash balances at a point in time."""

    positions: list[_types.Position]
    """Positions by account and instrument."""
    lots: list[_types.Lot]
    """Open lots, by account and instrument, in the order they were opened."""
    cash: list[_types.CashBalance]
    """Cash balances by account and currency."""


class Portfolio:
    """A portfolio: its records (base currency, accounts, instruments and transactions), the
    market data that values it, and the books kept from them. Everything Finalytics measures
    about a portfolio starts here.

    Read one from CSV files with :meth:`from_csv` (only the transactions are required), from a
    portfolio document with :meth:`read`, or build one from :class:`PortfolioData` and
    :class:`MarketData`. A portfolio does not change: :meth:`with_transactions` and
    :meth:`with_rebalance` return new ones.
    """

    def __init__(self, data: PortfolioData, market: MarketData | None = None):
        native = call(_finalytics.NativePortfolio, data.to_json(), (market or MarketData())._native)
        self._native = native
        self._data: PortfolioData | None = data
        self._market: MarketData | None = market

    @classmethod
    def _wrap(cls, native: Any) -> Portfolio:
        portfolio = cls.__new__(cls)
        portfolio._native = native
        portfolio._data = None
        portfolio._market = None
        return portfolio

    @classmethod
    def from_csv(
        cls,
        *,
        transactions: TextOrPath,
        prices: TextOrPath | None = None,
        fx: TextOrPath | None = None,
        instruments: TextOrPath | None = None,
        accounts: TextOrPath | None = None,
        goals: TextOrPath | None = None,
        base_currency: str | None = None,
        delimiter: str = ",",
    ) -> Portfolio:
        """A portfolio from CSV files, each given as a path or as text.

        - `transactions`: one row per transaction, with a ``date`` (or ``trade_date``), a
          ``type`` and the columns the type needs, such as ``instrument``, ``quantity`` and
          ``price`` for a ``buy``, or ``amount`` and ``currency`` for a ``deposit``. Rows may
          give an ``id`` (``t1``, ``t2`` and so on by default) and an ``account`` (``main``).
        - `prices`: a ``date`` column and one column per instrument, or ``date``,
          ``instrument`` and ``price`` columns.
        - `fx`: a ``date`` column and one column per pair, such as ``EUR/USD``, for instruments
          and cash in other currencies.
        - `instruments`: ``id``, ``kind`` and ``currency``, with ``class.<scheme>`` columns such
          as ``class.asset_class`` for grouping. Without it, each instrument is in the currency
          given with it in the transactions, or else in the base currency.
        - `accounts`: ``id``, with ``name``, ``lot_method`` and ``class.<scheme>`` columns.
          Without it, each account the transactions name is created.
        - `goals`: goals kept with the portfolio, one record per row: ``goal`` (its name) and
          ``type``, a ``target``, ``payments`` or ``projection`` row with the plan's settings,
          and ``saving``, ``spending``, ``mix``, ``alternative``, ``glide`` and
          ``alternative_glide`` rows; :attr:`goals` gives them, for :meth:`Assets.plan`.
        - `base_currency`: the currency results are reported in; by default, that of the first
          deposit.

        ``fl.datasets.file("portfolio/transactions.csv")`` and the other dataset files show each
        layout, and the data guide (https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) describes every column. Every problem in every
        file is reported at once, by line and column, in a :class:`~finalytics.ValidationError`.
        """
        native = call(
            _finalytics.NativePortfolio.from_csv,
            _text(transactions),
            delimiter,
            None if instruments is None else _text(instruments),
            None if accounts is None else _text(accounts),
            None if prices is None else _text(prices),
            None if fx is None else _text(fx),
            base_currency,
            None if goals is None else _text(goals),
        )
        return cls._wrap(native)

    @classmethod
    def read(cls, source: TextOrPath) -> Portfolio:
        """A portfolio from a portfolio document (``"format": "finalytics.portfolio"``), given
        as a path or as JSON text."""
        return cls._wrap(call(_finalytics.NativePortfolio.from_document, _text(source)))

    def to_json(self) -> str:
        """The portfolio document of the portfolio and its market data, as JSON text."""
        return self._native.to_document()

    @property
    def data(self) -> PortfolioData:
        """The records: base currency, accounts, instruments and transactions."""
        if self._data is None:
            self._data = PortfolioData.from_json(self._native.data())
        return self._data

    @property
    def market(self) -> MarketData:
        """The market data."""
        if self._market is None:
            self._market = MarketData._wrap(self._native.market())
        return self._market

    @property
    def base_currency(self) -> str:
        """The currency results are reported in."""
        return self._native.base_currency()

    @property
    def as_of(self) -> datetime.date | None:
        """The last date with a price: the date the portfolio is valued on when no other is
        given."""
        found = self._native.as_of()
        return None if found is None else datetime.date.fromisoformat(found)

    def dates(self) -> DateArray:
        """The dates the portfolio is measured on when none are given: every date with a price,
        from the first transaction to the last price."""
        return _arrays.dates_from_buffer(self._native.dates())

    def with_transactions(self, transactions: Iterable[_types.Transaction]) -> Portfolio:
        """The same portfolio with more transactions, applied after its own."""
        text = dumps(list(transactions))
        return Portfolio._wrap(call(self._native.with_transactions, text))

    def with_rebalance(self, proposal: _types.Rebalancing | _types.RebalanceResults) -> Portfolio:
        """The same portfolio with the trades of a rebalancing (from :meth:`rebalance`), or of
        its raw results, made on its date."""
        if isinstance(proposal, _types.Rebalancing):
            proposal = proposal.results
        return Portfolio._wrap(call(self._native.with_rebalance, dumps(proposal)))

    def with_market(self, market: MarketData) -> Portfolio:
        """The same portfolio valued with other market data."""
        return Portfolio._wrap(call(self._native.with_market, market._native))

    @property
    def goals(self) -> list[_types.PlanOptions]:
        """The goals kept with the portfolio, each a plan with its name, for
        :meth:`Assets.plan`: saved in the portfolio's document and read from a goals file."""
        return loads(list[_types.PlanOptions], self._native.goals())

    def with_goals(self, goals: Sequence[_types.PlanOptions]) -> Portfolio:
        """The same portfolio with `goals` kept with it in place of its own: each a plan with a
        name, no two the same; a goal without a currency takes the base currency."""
        return Portfolio._wrap(call(self._native.with_goals, dumps(list(goals), "goals")))

    def assets(self) -> Assets:
        """The holdings at the last price date, as :class:`~finalytics.Assets`: their returns in
        the base currency on the portfolio's dates, for statistics and optimisation. The
        instruments are the assets optimisation chooses among (``risky_assets``); the cash is there
        too (``cash_assets``), so that the portfolio's own :meth:`weights` can be valued and
        optimised from on the same estimates: base-currency cash, such as ``"USD cash"``, as the
        risk-free asset, riskless and earning nothing, and cash in other currencies, such as
        ``"EUR cash"``, moving with its exchange rate. The returns start on the first date every
        instrument has a price, as the overview report's estimates do."""
        from finalytics.assets import Assets

        return Assets._wrap(call(self._native.assets))

    def holdings(self, date: DateLike | None = None) -> Holdings:
        """Positions, lots and cash at the end of `date`, or after the last transaction."""
        found = json.loads(call(self._native.holdings, _arrays.optional_date(date)))
        return Holdings(
            positions=from_json(list[_types.Position], found["positions"]),
            lots=from_json(list[_types.Lot], found["lots"]),
            cash=from_json(list[_types.CashBalance], found["cash"]),
        )

    def valuation(self, date: DateLike | None = None) -> _types.Valuation:
        """The holdings valued at the end of `date` (by default the last price date). Missing
        prices and rates are an error; carried ones are listed in the valuation's warnings."""
        return loads(_types.Valuation, call(self._native.valuation, self._date_or_last_price(date)))

    def value_series(self, dates: Any = None) -> _types.ValueSeries:
        """End-of-day values and external flows on `dates` (strictly increasing)."""
        dates = self.dates() if dates is None else dates
        return loads(_types.ValueSeries, call(self._native.value_series, _arrays.dates_to_buffer(dates)))

    def holding_series(self, dates: Any = None) -> _types.HoldingSeries:
        """Each holding's daily value, trade cash and income on `dates`."""
        dates = self.dates() if dates is None else dates
        return loads(_types.HoldingSeries, call(self._native.holding_series, _arrays.dates_to_buffer(dates)))

    def account_series(self, dates: Any = None) -> _types.AccountSeries:
        """Each account's end-of-day value and external flows on `dates` (by default every price
        date from the first transaction)."""
        dates = self.dates() if dates is None else dates
        return loads(_types.AccountSeries, call(self._native.account_series, _arrays.dates_to_buffer(dates)))

    def household(
        self,
        dates: Any = None,
        *,
        group_by: Sequence[str] = (),
        timing: _types.FlowTiming = "start_of_day",
    ) -> _types.Household:
        """Performance over `dates` (by default every price date from the first transaction):
        every account together, each account, and the groups of accounts by each classification
        scheme in `group_by`, such as ``"owner"``. Each has its value, flows, time- and
        money-weighted returns, and its contribution to the household's time-weighted return;
        the accounts' contributions add up to that return."""
        dates = self.dates() if dates is None else dates
        options = _types.HouseholdOptions(group_by=list(group_by), timing=timing)
        text = call(self._native.household, _arrays.dates_to_buffer(dates), dumps(options, "options"))
        return loads(_types.Household, text)

    def realised_gains(self) -> list[_types.RealisedGain]:
        """Every realised gain, by lot, in the order the disposals happened."""
        return loads(list[_types.RealisedGain], self._native.realised_gains())

    def events(self) -> list[_types.Event]:
        """What each transaction did to the books, in the order applied."""
        return loads(list[_types.Event], self._native.events())

    def performance(self, dates: Any = None, *, timing: _types.FlowTiming = "start_of_day"):
        """Time-weighted performance measured on `dates` (by default, every price date from the
        first transaction), with external flows at the start or the end of the day."""
        from finalytics.performance import Performance

        dates = self.dates() if dates is None else dates
        return Performance._wrap(call(self._native.performance, _arrays.dates_to_buffer(dates), timing))

    def weights(self, date: DateLike | None = None) -> dict[_types.HoldingKey, float]:
        """Each holding's share of the portfolio's value at the end of `date` (by default the
        last price date), in the base currency."""
        return loads(dict[str, float], call(self._native.holding_weights, self._date_or_last_price(date)))

    def holding_returns(self, holdings: Iterable[_types.HoldingKey], dates: Any = None) -> _types.HoldingReturns:
        """Price returns of `holdings` (instrument identifiers, or ``"USD cash"`` and the like) in
        the base currency between consecutive `dates`, the input to covariance estimation."""
        dates = self.dates() if dates is None else dates
        text = call(self._native.holding_returns, dumps(list(holdings)), _arrays.dates_to_buffer(dates))
        return loads(_types.HoldingReturns, text)

    def lots_for_sale(
        self,
        date: DateLike,
        account: str,
        instrument: str,
        quantity: decimal.Decimal | int | str,
        method: _types.LotMethod = "fifo",
    ) -> list[_types.LotSale]:
        """The lots a sale would close, chosen by `method`, without changing the books."""
        text = call(self._native.lots_for_sale, _arrays.date(date), account, instrument, str(quantity), method)
        return loads(list[_types.LotSale], text)

    def rebalance(
        self,
        targets: Mapping[_types.HoldingKey, float] | _types.Optimisation | _types.OptimisationResult,
        *,
        accounts: Sequence[str] | None = None,
        as_of: DateLike | None = None,
        lot_method: _types.LotMethod | None = None,
        whole_units: bool = False,
        minimum_trade: float = 0,
        fee_bps: float = 0,
        fee_per_trade: float = 0,
        minimum_fee: float = 0,
        tax: tuple[float, float] | None = None,
        long_term_after_days: int | None = None,
        keep_cash: bool | Sequence[str] = False,
        name: str | None = None,
    ) -> _types.Rebalancing:
        """The trades that move `accounts` (by default every account) together to `targets`:
        weights by holding (an instrument's identifier, or ``"USD cash"`` and the like), as
        shares of their combined value on `as_of` (by default the last price date), or an
        optimisation of the portfolio's assets, whose weights are its targets (one that found
        no weights is a :class:`ValidationError`). The rebalance report is in
        ``result.report``, which ``print(result)`` prints and ``result.show()`` shows, and each
        account's proposal in ``result.results``; :meth:`with_rebalance` makes the trades.
        `name` titles the report.

        Instruments held but not listed are sold; base-currency cash takes the weight the
        targets leave, and other currencies are converted to their targets (zero if not
        listed). A holding above its target is sold from the accounts that hold it in
        proportion to their holdings, and one below is bought in proportion to the value each
        account has free; trades do not move cash between accounts. Sales take lots by
        `lot_method`, by default each account's own. Trades worth less than `minimum_trade` in
        the base currency are left out.

        Each trade pays a fee of `fee_bps` basis points of its value plus `fee_per_trade`, and
        at least `minimum_fee`, in the instrument's currency; purchases leave room for the fees.
        With `tax`, the rates on short-term and long-term gains such as ``(0.37, 0.20)``, sales
        in taxable accounts take the lots that cost the least tax, and the tax is estimated; a
        gain is long-term after `long_term_after_days` (365 by default), and an account that is
        not taxable, such as a retirement account, pays none.

        ``keep_cash=True`` keeps every currency's cash as it is, and ``keep_cash=["USD"]`` that
        of the currencies named: the other targets then share the rest of the value, so the
        trades neither invest that cash nor add to it. Targets that leave the kept cash out are
        shares of the rest; targets that give it its weight now, as an optimisation with the same
        `keep_cash` gives them, are shares of the whole value and are traded as they are. By
        default the targets are shares of the whole value, cash included.
        """
        if isinstance(keep_cash, str):
            raise invalid(("keep_cash", f"must be True or a list of currencies, such as [{keep_cash!r}]"))
        choices = {
            "accounts": [a.id for a in self.data.accounts] if accounts is None else list(accounts),
            "as_of": self._date_or_last_price(as_of),
            "name": name,
            "lot_method": lot_method,
            "whole_units": whole_units,
            "minimum_trade": float(minimum_trade),
            "fee_bps": float(fee_bps),
            "fee_per_trade": float(fee_per_trade),
            "minimum_fee": float(minimum_fee),
            "tax": None if tax is None else [float(tax[0]), float(tax[1])],
            "long_term_after_days": long_term_after_days,
            "keep_cash": keep_cash if isinstance(keep_cash, bool) else list(keep_cash),
        }
        if isinstance(targets, _types.Optimisation):
            # The method and limits name the targets in the report.
            text = dumps({"report": {"context": targets.report.context}, "results": targets.results})
        elif isinstance(targets, _types.OptimisationResult):
            text = dumps(targets)
        else:
            text = dumps({k: float(w) for k, w in targets.items()})
        result = call(self._native.rebalance_accounts, text, dumps(choices, "choices"))
        return loads(_types.Rebalancing, result)

    def shock_scenario(
        self,
        *,
        instruments: Mapping[str, float] | None = None,
        groups: Mapping[tuple[str, str], float] | None = None,
        currencies: Mapping[str, float] | None = None,
        factors: Mapping[str, float] | None = None,
        factor_model: _types.FactorModel | _types.FactorModelEstimate | None = None,
        propagate: bool = False,
        date: DateLike | None = None,
        group_by: Sequence[str] = (),
        risk: _types.Matrix | _types.FactorModel | _types.FactorModelEstimate | None = None,
        confidence: float = 0.95,
        stress: _types.CorrelationStress | None = None,
    ) -> _types.ScenarioResult:
        """What shocks would do to the holdings on `date` (by default the last price date).

        `instruments` and `currencies` map identifiers and currency codes to returns, and
        `groups` maps `(scheme, value)` pairs, such as `("asset_class", "Equity")`, to the return
        of every instrument in the group. `factors` maps factor names of `factor_model` to
        returns; with `propagate`, the other factors move by their expected return given them.
        An instrument's own shock comes first, then the one group that matches it (more than one
        is an error), then the factors; an instrument none of them reaches is listed as not
        shocked. Currency shocks compound with an instrument's return and move cash.

        With `risk` (a covariance matrix or factor model of the holdings' returns, labelled by
        instrument and "EUR cash" style labels), the result has the risk before and after, the
        latter with the covariance stressed by `stress`.
        """
        factor_shocks = None
        if factors:
            if factor_model is None:
                raise invalid(("factor_model", "factor shocks need a factor model"))
            model = factor_model.model if isinstance(factor_model, _types.FactorModelEstimate) else factor_model
            factor_shocks = _types.FactorShocks(
                model=model, shocks={k: float(v) for k, v in factors.items()}, propagate=propagate
            )
        shocks = _types.Shocks(
            instruments={k: float(v) for k, v in (instruments or {}).items()},
            groups=[
                _types.GroupShock(scheme=scheme, value=value, shock=float(v))
                for (scheme, value), v in (groups or {}).items()
            ],
            currencies={k: float(v) for k, v in (currencies or {}).items()},
            factors=factor_shocks,
        )
        options = self._scenario_options(group_by, risk, confidence, stress)
        text = call(self._native.shock_scenario, self._date_or_last_price(date), dumps(shocks), dumps(options))
        return loads(_types.ScenarioResult, text)

    def historical_scenario(
        self,
        start: DateLike,
        end: DateLike,
        *,
        proxies: Mapping[str, str] | None = None,
        date: DateLike | None = None,
        group_by: Sequence[str] = (),
        risk: _types.Matrix | _types.FactorModel | _types.FactorModelEstimate | None = None,
        confidence: float = 0.95,
        stress: _types.CorrelationStress | None = None,
    ) -> _types.ScenarioResult:
        """What the price and exchange-rate changes from `start` to `end` would do to the
        holdings on `date` (by default the last price date), with the holdings' value on each
        date in between. `proxies` maps instruments with no prices then to instruments whose
        prices stand in for them; their own currency's rate still applies. `group_by`, `risk`,
        `confidence` and `stress` are as for :meth:`shock_scenario`."""
        window = _types.HistoricalWindow(
            start=datetime.date.fromisoformat(_arrays.date(start)),
            end=datetime.date.fromisoformat(_arrays.date(end)),
            proxies=dict(proxies or {}),
        )
        options = self._scenario_options(group_by, risk, confidence, stress)
        text = call(self._native.historical_scenario, self._date_or_last_price(date), dumps(window), dumps(options))
        return loads(_types.ScenarioResult, text)

    @staticmethod
    def _scenario_options(
        group_by: Sequence[str],
        risk: _types.Matrix | _types.FactorModel | _types.FactorModelEstimate | None,
        confidence: float,
        stress: _types.CorrelationStress | None,
    ) -> _types.ScenarioOptions:
        model = None
        if isinstance(risk, _types.FactorModelEstimate):
            risk = risk.model
        if isinstance(risk, _types.Matrix):
            model = _types.ScenarioRiskModel(covariance=risk, confidence=confidence, stress=stress)
        elif isinstance(risk, _types.FactorModel):
            model = _types.ScenarioRiskModel(factor_model=risk, confidence=confidence, stress=stress)
        elif risk is not None:
            raise TypeError("risk must be a covariance matrix or a factor model")
        return _types.ScenarioOptions(group_by=list(group_by), risk=model)

    def _date_or_last_price(self, date: DateLike | None) -> str:
        if date is None:
            date = self.as_of
            if date is None:
                raise invalid(("date", "give a date: the portfolio has no prices"))
        return _arrays.date(date)

    def report(
        self,
        options: _types.ReportOptions | None = None,
        *,
        name: str | None = None,
        **keywords: Any,
    ) -> _types.PortfolioReport:
        """Every report on the portfolio, from one set of options: the fields of
        :class:`~finalytics.reports.ReportOptions`, given as keywords or as a ``ReportOptions``,
        such as ``benchmark="SPX"``, ``group_by=["asset_class"]`` and ``risk_free_rate=0.03``.
        The attribution report needs ``attribution`` (the benchmark's segments) and the
        scenarios report needs ``scenarios``; without them, they are ``None``. `name`, such as
        ``"Morgan family portfolio"``, is shown under the report's title.

        ``report.show()`` shows the report inline in a notebook or in a browser, and
        ``report.save_html(path)`` saves it as one page."""
        from finalytics.reports import _options

        text = call(self._native.report_all, dumps(_options(options, keywords), "options"))
        report = loads(_types.PortfolioReport, text)
        return report if name is None else dataclasses.replace(report, name=name)

    def overview_report(self, options: _types.ReportOptions | None = None, **keywords: Any) -> _types.OverviewReport:
        """Headline figures, value and growth against the benchmark, allocation, risk by
        holding and by group, each holding's expected return and risk, and the efficient
        frontier with the current portfolio."""
        from finalytics.reports import _report

        return _report(self, "overview", _types.OverviewReport, options, keywords)

    def holdings_report(self, options: _types.ReportOptions | None = None, **keywords: Any) -> _types.HoldingsReport:
        """Positions, lots and cash by holding and by account; allocation by holding, currency,
        instrument kind, account and each scheme of ``group_by``; concentration; and realised
        gains and income to date."""
        from finalytics.reports import _report

        return _report(self, "holdings", _types.HoldingsReport, options, keywords)

    def performance_report(
        self, options: _types.ReportOptions | None = None, **keywords: Any
    ) -> _types.PerformanceReport:
        """Value and growth over time, returns over the standard periods against the benchmark,
        monthly and calendar-year returns, contribution by holding and by group, flows and
        income by month, and the household by account."""
        from finalytics.reports import _report

        return _report(self, "performance", _types.PerformanceReport, options, keywords)

    def risk_report(self, options: _types.ReportOptions | None = None, **keywords: Any) -> _types.RiskReport:
        """Statistics, drawdowns, value at risk and expected shortfall by every method, the
        return distribution, the holdings' covariance, correlation and risk decomposition, and
        rolling statistics."""
        from finalytics.reports import _report

        return _report(self, "risk", _types.RiskReport, options, keywords)

    def attribution_report(
        self, options: _types.ReportOptions | None = None, **keywords: Any
    ) -> _types.AttributionReport:
        """Brinson–Fachler attribution against the benchmark segments given as
        ``attribution``, with the portfolio's segments measured from its books."""
        from finalytics.reports import _report

        return _report(self, "attribution", _types.AttributionReport, options, keywords)

    def scenarios_report(self, options: _types.ReportOptions | None = None, **keywords: Any) -> _types.ScenariosReport:
        """The ``scenarios`` given, run together on the current holdings with the holdings'
        covariance as the risk model, side by side."""
        from finalytics.reports import _report

        return _report(self, "scenarios", _types.ScenariosReport, options, keywords)

    def __repr__(self) -> str:
        data = self.data
        return (
            f"Portfolio({len(data.transactions)} transactions, "
            f"{len(data.accounts)} accounts, base {self.base_currency})"
        )


def to_json_data(value: Any) -> Any:
    """`value` as JSON data in the engine's formats."""
    return to_json(value)
