# Valuation and FX rates

A valuation prices every position and converts it, with every cash balance, into the base
currency. Prices and rates are supplied by the caller; Finalytics fetches nothing.

## Definitions

A position's market value is $q \times p \times m$ (quantity, price, multiplier) in the
instrument's currency, and $q \times p \times m \times x$ in the base currency, where $x$ is the
number of base-currency units per unit of the instrument's currency. A cash balance's value is
$a \times x$. Unrealised gains are market value less cost, in each currency; in the base currency
they include the currency effect.

### Finding an FX rate

A rate is the number of units of `quote` per unit of `base`. For a pair, Finalytics looks for, in
order:

1. the rate itself;
2. the inverse pair, as $1 / x$;
3. both legs through the **pivot** currency (USD by default), as
   $x_{\text{base} \to \text{pivot}} \times x_{\text{pivot} \to \text{quote}}$, each leg found
   directly or inverted.

A currency's rate against itself is 1.

### Carrying values forward

When a date has no price or rate, the latest earlier value is used if it is at most
`max_carry_days` calendar days old (5 by default), which covers weekends and holidays. Such a
value is **carried**: the result notes the date it was observed, and valuations list every
carried value as a warning. A value older than the limit is **missing**, and a valuation with a
missing value is an error that lists every missing price and rate at once.

## Conventions

- Prices are per unit in the instrument's currency; a bond's price is per unit of face value as
  the caller defines it, with the multiplier converting it to value.
- Values are exact decimals; the base-currency value of a position is converted at the rate on
  the valuation date.

## Example

```python
>>> import finalytics as fl
>>> market = fl.MarketData(
...     fx=[
...         ("EUR", "USD", {"2026-06-30": "1.10"}),
...         ("GBP", "USD", {"2026-06-30": "1.25"}),
...     ],
... )
>>> market.fx_rate("EUR", "GBP", "2026-06-30").value      # crossed through USD
Decimal('0.880')
>>> found = market.fx_rate("EUR", "USD", "2026-07-03")     # three days later: carried
>>> (found.value, found.date, found.carried)
(Decimal('1.10'), datetime.date(2026, 6, 30), True)
>>> print(market.fx_rate("EUR", "USD", "2026-07-08"))      # eight days later: missing
None

```

## Validation

Rates found directly, inverted and crossed, and carried and missing values, are checked against
hand-worked cases in `crates/finalytics-ledger/src/market.rs`; valuations of foreign holdings, in
both currencies, in `crates/finalytics-ledger/tests/books.rs`. A valuation's total is checked to
equal the sum of its positions and cash exactly, in every language's tests.
