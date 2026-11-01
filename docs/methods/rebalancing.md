# Rebalancing

Rebalancing turns target weights into the trades that move one account from its current holdings
to those weights, choosing the lots each sale closes and the currency conversions that fund
purchases in other currencies.

## Method

On a date $D$, with the account's value $V$ in the base currency (positions at $D$'s prices and
rates, plus cash):

1. **Instruments.** For each instrument held or targeted, with unit value $u = p \times m \times
   x$ in the base currency, the trade is $(w V - q u) / u$ units, where $q$ is the quantity held
   and $w$ the target (zero for an instrument held but not listed, which is sold entirely). With
   whole units, each trade is rounded towards zero. Sales are capped at the quantity held, and
   trades worth less than the minimum trade are left out (and noted in the warnings).
2. **Lots.** Each sale closes lots by the chosen method (first in, first out by default; highest
   cost first defers gains), exactly as the ledger would, and records the gain it would realise
   in the base currency before fees.
3. **Cash.** Trades change the cash in each instrument's currency. Each foreign currency is then
   converted, through the base currency, to its target balance (zero when not listed), rounded to
   the currency's minor units. Cash in the base currency takes whatever weight the targets leave.
4. **Result.** The proposal lists the sales, then the purchases, and the conversions, with the
   weights before and after, the turnover (the value traded, sales and purchases, over $V$), the
   realised gain, and warnings for carried prices or a shortfall of base-currency cash.

`transactions(plan)` turns the proposal into ledger transactions on $D$ (sales naming their lots,
then conversions, then purchases), so that applying it to the books gives the portfolio after the
trades.

## Conventions

- One account at a time; trades are not placed across accounts.
- Only long positions are proposed; the target weights must be zero or more and add up to at
  most one.
- Fees are not modelled; the realised gains are before fees.

## Example

```python
>>> import finalytics as fl
>>> portfolio = fl.Portfolio.from_csv(
...     instruments="id,kind,currency\nSPY,fund,USD\nBOND,bond,USD\nSAP,equity,EUR\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency\n"
...         "t1,main,2026-03-02,deposit,,,,100000,USD\n"
...         "t2,main,2026-03-02,buy,SPY,100,400,,\n"
...         "t3,main,2026-03-03,buy,SPY,50,450,,\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(
...     prices={"SPY": {"2026-03-10": 500}, "BOND": {"2026-03-10": 100}, "SAP": {"2026-03-10": 210}},
...     fx=[("EUR", "USD", {"2026-03-02": "1.10", "2026-03-10": "1.10"})],
... )
>>> plan = portfolio.books(market).rebalance(
...     "main", {"SPY": 0.5, "BOND": 0.3, "SAP": 0.15}, lot_method="highest_cost", whole_units=True
... )
>>> plan.value_base, round(plan.turnover, 6), plan.realised_gain_base
(Decimal('112500'), 0.613893, Decimal('1850'))
>>> for trade in plan.trades:
...     print(trade.side, trade.instrument, trade.quantity, trade.value_base, [(l.lot, l.quantity) for l in trade.lots])
sell SPY 37 18500 [('t3', Decimal('37'))]
buy BOND 337 33700 []
buy SAP 73 16863.00 []
>>> [(c.sell.amount, c.sell.currency, c.buy.amount, c.buy.currency) for c in plan.conversions]
[(Decimal('16863.00'), 'USD', Decimal('15330'), 'EUR')]
>>> [t.kind.TYPE for t in fl.opt.transactions(plan)]
['sell', 'fx_conversion', 'buy', 'buy']

```

The account is worth 112,500, so SPY's target is 56,250, or 112.5 units: selling 37.5 rounds to
37. Highest cost first takes them from the lot bought at 450.

## Validation

`crates/finalytics-opt/tests/rebalance.rs` builds a two-account, two-currency portfolio, proposes
trades, applies them as transactions, and rebuilds the books: the rebuilt account reaches its
targets to within the rounding of currency conversions, its realised gains match the proposal's,
the proposal's own weights after the trades match the rebuilt books to $10^{-12}$, the euro
balance is spent to within half a cent, and the other account is untouched. Whole units, minimum
trades, sales of everything not targeted, turnover and invalid targets are checked as well.
