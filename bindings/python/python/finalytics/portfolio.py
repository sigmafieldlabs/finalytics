"""Portfolios, market data and the books kept from them."""

from __future__ import annotations

import dataclasses
import datetime
import decimal
import json
import math
import os
import pathlib
from collections.abc import Iterable, Mapping, Sequence
from typing import Any, NamedTuple

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._arrays import DateArray, DateLike
from finalytics._convert import dumps, from_json, loads, to_json
from finalytics._errors import call

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


def holding(instrument: str) -> _types.InstrumentHolding:
    """The holding of an instrument, added up across accounts."""
    return _types.InstrumentHolding(id=instrument)


def cash(currency: str) -> _types.CashHolding:
    """The holding of cash in one currency, added up across accounts."""
    return _types.CashHolding(id=currency)


def _holding_key(key: str | _types.HoldingKey) -> _types.HoldingKey:
    return holding(key) if isinstance(key, str) else key


class Portfolio(_types.Portfolio):
    """Everything Finalytics needs to keep a portfolio's books: its base currency, accounts,
    instruments and transactions.

    Build one from dataclasses, from JSON, from a portfolio document, or from CSV files.
    """

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Portfolio:
        """A portfolio from decoded JSON in the portfolio format."""
        return from_json(cls, data)

    @classmethod
    def from_json(cls, text: str) -> Portfolio:
        """A portfolio from JSON text in the portfolio format (the ``portfolio`` member of a
        portfolio document)."""
        return loads(cls, text)

    @classmethod
    def from_csv(
        cls,
        *,
        instruments: TextOrPath,
        transactions: TextOrPath,
        base_currency: str,
        delimiter: str = ",",
    ) -> Portfolio:
        """A portfolio from CSV files of instruments and transactions, with one account per
        account named in the transactions.

        Every problem is reported at once, by line and column, in a
        :class:`~finalytics.ValidationError`.
        """
        text = call(
            _finalytics.read_portfolio_csv,
            base_currency,
            _text(instruments),
            _text(transactions),
            delimiter,
        )
        return cls.from_json(text)

    def to_json(self) -> str:
        """The portfolio as JSON text."""
        return dumps(self)

    def validate(self) -> list[_types.Issue]:
        """Every structural problem in the portfolio, without applying its transactions.

        Problems found only by applying the transactions in order, such as selling more than is
        held, are reported when the books are built.
        """
        report = loads(_types.ValidationReport, call(_finalytics.validate_portfolio, self.to_json()))
        return report.issues

    def books(self, market: MarketData | None = None) -> Books:
        """Keeps the books: applies the transactions, using `market` for FX rates and for the
        prices that transfers without a cost basis need."""
        return Books(self, market or MarketData())


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
        raise ValueError(f"prices and rates must be finite numbers, found {number}")
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
    """A portfolio document: a portfolio and, optionally, its market data."""

    portfolio: Portfolio
    market: MarketData | None


def read_document(source: TextOrPath) -> Document:
    """Reads a portfolio document (``"format": "finalytics.portfolio"``), reporting where a
    malformed one goes wrong."""
    portfolio, market = call(_finalytics.read_document, _text(source))
    return Document(Portfolio.from_json(portfolio), None if market is None else MarketData._wrap(market))


def write_document(portfolio: Portfolio, market: MarketData | None = None) -> str:
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


class Books:
    """The books of a portfolio: its transactions applied in order, with the market data used to
    value them."""

    def __init__(self, portfolio: Portfolio, market: MarketData):
        self.portfolio = portfolio
        """The portfolio."""
        self.market = market
        """The market data used for valuation."""
        self._native = call(_finalytics.NativeBooks, portfolio.to_json(), market._native)

    @property
    def base_currency(self) -> str:
        """The currency results are reported in."""
        return self._native.base_currency()

    def default_dates(self) -> DateArray:
        """The dates performance is measured on when none are given: every date with a price,
        from the first transaction to the last price."""
        dates = self.market.price_dates
        if not self.portfolio.transactions:
            return dates
        first = np.datetime64(min(t.trade_date for t in self.portfolio.transactions), "D")
        return dates[dates >= first]

    def holdings(self, date: DateLike | None = None) -> Holdings:
        """Positions, lots and cash at the end of `date`, or after the last transaction."""
        found = json.loads(call(self._native.holdings, _arrays.optional_date(date)))
        return Holdings(
            positions=from_json(list[_types.Position], found["positions"]),
            lots=from_json(list[_types.Lot], found["lots"]),
            cash=from_json(list[_types.CashBalance], found["cash"]),
        )

    def valuation(self, date: DateLike) -> _types.Valuation:
        """The holdings valued at the end of `date`. Missing prices and rates are an error;
        carried ones are listed in the valuation's warnings."""
        return loads(_types.Valuation, call(self._native.valuation, _arrays.date(date)))

    def value_series(self, dates: Any = None) -> _types.ValueSeries:
        """End-of-day values and external flows on `dates` (strictly increasing)."""
        dates = self.default_dates() if dates is None else dates
        return loads(_types.ValueSeries, call(self._native.value_series, _arrays.dates_to_buffer(dates)))

    def holding_series(self, dates: Any = None) -> _types.HoldingSeries:
        """Each holding's daily value, trade cash and income on `dates`."""
        dates = self.default_dates() if dates is None else dates
        return loads(_types.HoldingSeries, call(self._native.holding_series, _arrays.dates_to_buffer(dates)))

    def realised_gains(self) -> list[_types.RealisedGain]:
        """Every realised gain, by lot, in the order the disposals happened."""
        return loads(list[_types.RealisedGain], self._native.realised_gains())

    def events(self) -> list[_types.Event]:
        """What each transaction did to the books, in the order applied."""
        return loads(list[_types.Event], self._native.events())

    def performance(self, dates: Any = None, *, timing: _types.FlowTiming = "start_of_day"):
        """Time-weighted performance measured on `dates` (by default, every price date from the
        first transaction), with flows at the start or the end of the day."""
        from finalytics.performance import Performance

        dates = self.default_dates() if dates is None else dates
        return Performance._wrap(call(self._native.performance, _arrays.dates_to_buffer(dates), timing))

    def weights(self, date: DateLike) -> dict[_types.HoldingKey, float]:
        """Each holding's share of the portfolio's value on `date`, in the base currency."""
        pairs = loads(list[tuple[_types.HoldingKey, float]], call(self._native.holding_weights, _arrays.date(date)))
        return dict(pairs)

    def holding_returns(self, holdings: Iterable[str | _types.HoldingKey], dates: Any = None) -> _types.HoldingReturns:
        """Price returns of `holdings` in the base currency between consecutive `dates`, the
        input to covariance estimation."""
        dates = self.default_dates() if dates is None else dates
        keys = [_holding_key(h) for h in holdings]
        text = call(self._native.holding_returns, dumps(keys), _arrays.dates_to_buffer(dates))
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
        account: str,
        targets: Mapping[str | _types.HoldingKey, float],
        *,
        date: DateLike | None = None,
        lot_method: _types.LotMethod = "fifo",
        whole_units: bool = False,
        minimum_trade: decimal.Decimal | int | str = 0,
        fees: _types.Fees | None = None,
        tax: _types.TaxRates | None = None,
    ) -> _types.Rebalance:
        """The trades that move `account` to `targets`: weights by instrument identifier (or by
        holding, for cash), as shares of the account's value on `date` (by default the last
        price date).

        Instruments held but not listed are sold; base-currency cash takes the weight the targets
        leave, and other currencies are converted to their targets (zero if not listed). Each
        trade pays `fees`; with `tax` rates, sales take the lots that cost the least tax, and the
        tax is estimated.
        """
        options = self._rebalance_options(lot_method, whole_units, minimum_trade, fees, tax)
        pairs = [[_holding_key(k), float(w)] for k, w in targets.items()]
        text = call(self._native.rebalance, self._date_or_last_price(date), account, dumps(pairs), dumps(options))
        return loads(_types.Rebalance, text)

    def rebalance_accounts(
        self,
        accounts: Sequence[str],
        targets: Mapping[str | _types.HoldingKey, float],
        *,
        date: DateLike | None = None,
        lot_method: _types.LotMethod = "fifo",
        whole_units: bool = False,
        minimum_trade: decimal.Decimal | int | str = 0,
        fees: _types.Fees | None = None,
        tax: _types.TaxRates | None = None,
    ) -> _types.CombinedRebalance:
        """The trades that move several `accounts` together to `targets`, as shares of their
        combined value: one proposal per account. A holding above its target is sold from the
        accounts that hold it in proportion to their holdings, and one below is bought in
        proportion to the value each account has free; trades do not move cash between
        accounts."""
        options = self._rebalance_options(lot_method, whole_units, minimum_trade, fees, tax)
        pairs = [[_holding_key(k), float(w)] for k, w in targets.items()]
        text = call(
            self._native.rebalance_accounts,
            self._date_or_last_price(date),
            json.dumps(list(accounts)),
            dumps(pairs),
            dumps(options),
        )
        return loads(_types.CombinedRebalance, text)

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
                raise ValueError("factor shocks need a factor_model")
            model = factor_model.model if isinstance(factor_model, _types.FactorModelEstimate) else factor_model
            factor_shocks = _types.FactorShocks(
                model=model, shocks=[(k, float(v)) for k, v in factors.items()], propagate=propagate
            )
        shocks = _types.Shocks(
            instruments=[(k, float(v)) for k, v in (instruments or {}).items()],
            groups=[
                _types.GroupShock(scheme=scheme, value=value, shock=float(v))
                for (scheme, value), v in (groups or {}).items()
            ],
            currencies=[(k, float(v)) for k, v in (currencies or {}).items()],
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
            proxies=list((proxies or {}).items()),
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
            dates = self.market.price_dates
            if len(dates) == 0:
                raise ValueError("give a date: the market data has no prices")
            date = dates[-1]
        return _arrays.date(date)

    @staticmethod
    def _rebalance_options(
        lot_method: _types.LotMethod,
        whole_units: bool,
        minimum_trade: decimal.Decimal | int | str,
        fees: _types.Fees | None,
        tax: _types.TaxRates | None,
    ) -> _types.RebalanceOptions:
        return _types.RebalanceOptions(
            lot_method=lot_method,
            whole_units=whole_units,
            minimum_trade=decimal.Decimal(str(minimum_trade)),
            fees=fees or _types.Fees(),
            tax=tax,
        )

    def __repr__(self) -> str:
        return (
            f"Books({len(self.portfolio.transactions)} transactions, "
            f"{len(self.portfolio.accounts)} accounts, base {self.base_currency})"
        )


def to_json_data(value: Any) -> Any:
    """`value` as JSON data in the engine's formats."""
    return to_json(value)
