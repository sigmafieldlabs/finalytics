# Rebalancing

Rebalancing turns target weights into the trades that move an account, or several accounts
together, from their current holdings to those weights, choosing the lots each sale closes, the
fees each trade pays and the currency conversions that fund purchases in other currencies.

## Method

On a date $D$, with the account's value $V$ in the base currency (positions at $D$'s prices and
rates, plus cash):

1. **Instruments.** For each instrument held or targeted, with unit value $u = p \times m \times
   x$ in the base currency, the trade is $(w V - q u) / u$ units, where $q$ is the quantity held
   and $w$ the target (zero for an instrument held but not listed, which is sold entirely). With
   whole units, each trade is rounded towards zero. Sales are capped at the quantity held, and
   trades worth less than the minimum trade are left out (and noted in the warnings).
2. **Fees.** Each trade pays $\max(\text{minimum}, \text{per trade} + \text{rate} \times
   \text{value})$ in the instrument's currency, rounded to its minor units (no fees by default).
   A purchase is reduced so that it and its fee fit the target; a sale pays its fee from the
   proceeds. When the trades and their fees would need more base-currency cash than the account
   has, as when the targets leave no cash for the fees, the purchases are sized again on $V$ less
   the shortfall, and a shortfall that rounding leaves is taken from the largest purchase, so the
   trades fit the cash the account has.
3. **Lots.** Each sale closes lots by the chosen method (by default the account's own, first in,
   first out unless the account says otherwise; highest cost first defers gains), or with tax
   rates by the tax they cost (below), exactly as the ledger would, and records the gain it
   would realise in the base currency after fees: the proceeds are split across its lots and
   rounded as the books split them, so applying the proposal realises exactly the gains it
   reports.
4. **Cash.** Trades change the cash in each instrument's currency. Each foreign currency is then
   converted, through the base currency, to its target balance (zero when not listed), rounded to
   the currency's minor units. Cash in the base currency takes whatever weight the targets leave.

   To keep cash as it is (`keep_cash`, for every currency or those named), the targets are read
   as shares of the value other than the cash kept: each weight $w_i$ becomes $w_i (1 - k)$,
   with $k$ the kept cash's share of $V$, and each kept balance takes its own share as its
   target. So the trades neither invest the cash nor add to it, other than by the rounding of
   units and conversions; a target that names kept cash is refused, and the report lists what
   was kept.
5. **Result.** The proposal lists the sales, then the purchases, and the conversions, with the
   weights before and after, the turnover (one-way: half the value traded, sales and purchases,
   over $V$), the realised gain, the fees, the estimated tax, and warnings for carried prices or a
   shortfall of base-currency cash. The rebalance report sets it out with the weights before, at the target
   and after, and the volatility and expected return before and after, from the holdings'
   annualised returns with the Ledoit–Wolf covariance and historical means, cash riskless.

`transactions(plan)` turns the proposal into ledger transactions on $D$ (sales naming their lots,
then conversions, then purchases, each with its fee), so that applying it to the books gives the
portfolio after the trades.

### Tax-aware sales

With tax rates, a short-term rate $t_s$, a long-term rate $t_l$ and a holding period $H$ (365 days
by default), each lot's tax per unit sold at $D$ is $t (u - c)$, where $c$ is its cost per unit in
the base currency and $t = t_l$ if it has been held for more than $H$ days, $t_s$ otherwise. A sale
takes lots in increasing order of that tax: losses first, which save the most, then the gains
taxed least; ties go to the lot opened first. Each sale reports its estimated tax, $\sum t_k
g_k$ over its lots' gains $g_k$ after fees, negative when its losses save more than its gains
cost, the part of its gain from lots held long-term, and the proposal reports the total. An
account that is not taxable (`taxable` false in its record), such as a retirement account, pays no
tax: its sales close lots by the lot method and report no tax. The rates are the caller's: no
particular tax code's rules (such as wash sales) are applied.

### Several accounts

Rebalancing several accounts (`Portfolio::rebalance` in Rust, `portfolio.rebalance(...)` in
Python and JavaScript, which take every account by default) moves them together to target weights
of their combined value
$V = \sum_a V_a$. Each holding $k$ with target value $T_k = w_k V$ is spread across the accounts
by a simple rule:

- if the accounts hold more than $T_k$ in total, each keeps the same share of its holding,
  $T_k / H_k$, where $H_k$ is the total held;
- otherwise each keeps what it holds, and the shortfall $T_k - H_k$ is bought in proportion to
  each account's free value: its value less what it keeps.

Each account is then rebalanced to its share, with the same options, so its sales choose lots,
pay fees and estimate tax as they would alone, and its purchases are made with its own cash. Trades
do not move cash between accounts. The result lists each account's proposal, with the combined
weights before and after, turnover, realised gains, fees and tax.

## Conventions

- Only long positions are proposed; the target weights must be zero or more and add up to at
  most one.
- Realised gains, fees and tax are in the base currency; gains are after fees.

## Example

```python
>>> import finalytics as fl
>>> data = fl.PortfolioData.from_csv(
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
>>> portfolio = fl.Portfolio(data, market)
>>> proposal = portfolio.rebalance(
...     {"SPY": 0.5, "BOND": 0.3, "SAP": 0.15}, lot_method="highest_cost", whole_units=True
... )
>>> [plan] = proposal.results.accounts
>>> plan.value_base, round(plan.turnover, 6), plan.realised_gain_base
(Decimal('112500'), 0.306947, Decimal('1850'))
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

With tax rates, a sale takes the lots that cost the least tax. Here an account holds three lots
of SPY, now at 500: one bought at 300 more than two years ago, one at 520 and one at 450 this year.

```python
>>> data = fl.PortfolioData.from_csv(
...     instruments="id,kind,currency\nSPY,fund,USD\nBOND,bond,USD\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency\n"
...         "t1,main,2024-01-02,deposit,,,,150000,USD\n"
...         "t2,main,2024-01-02,buy,SPY,100,300,,\n"
...         "t3,main,2026-01-05,buy,SPY,100,520,,\n"
...         "t4,main,2026-02-02,buy,SPY,100,450,,\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(prices={"SPY": {"2026-03-10": 500}, "BOND": {"2026-03-10": 100}})
>>> portfolio = fl.Portfolio(data, market)
>>> targets = {"SPY": 0.4, "BOND": 0.5}
>>> [fifo] = portfolio.rebalance(targets, whole_units=True).results.accounts
>>> [(l.lot, l.quantity) for l in fifo.trades[0].lots], fifo.realised_gain_base
([('t2', Decimal('100')), ('t3', Decimal('61'))], Decimal('18780'))
>>> result = portfolio.rebalance(targets, whole_units=True, tax=(0.4, 0.2), fee_bps=5, minimum_fee=5)
>>> [plan] = result.results.accounts
>>> [(l.lot, l.quantity) for l in plan.trades[0].lots], plan.realised_gain_base, plan.tax_base, plan.fees_base
([('t3', Decimal('100')), ('t4', Decimal('61'))], Decimal('1009.75'), Decimal('403.900'), Decimal('83.45'))

```

First in, first out sells the long-held lot and realises 18,780, which would cost 3,512 of tax:
4,000 on its long-term gain, less 488 saved on the 1,220 lost on t3. The tax-aware sale takes the
lot at a loss first and then the cheapest gain, realising 1,009.75 after fees, which costs 403.90
at the short-term rate.

## Validation

`crates/finalytics-opt/tests/rebalance.rs` builds a two-account, two-currency portfolio, proposes
trades, applies them as transactions, and rebuilds the books: the rebuilt account reaches its
targets to within the rounding of currency conversions, its realised gains match the proposal's,
the proposal's own weights after the trades match the rebuilt books to $10^{-12}$, the euro
balance is spent to within half a cent, and the other account is untouched. Whole units, minimum
trades, sales of everything not targeted, turnover and invalid targets are checked as well.

With fees, each trade's fee follows the schedule, the books record exactly the realised gains the
proposal reports, and no account is overdrawn. Tax-aware sales are checked on lots with a
long-term gain, a short-term loss and a short-term gain, against hand-computed lots, gains and
tax. Rebalancing several accounts together reaches the combined targets to $10^{-7}$, sells the
same share of a holding from each account that holds it, and, applied to the books, reaches the
targets to $10^{-6}$ without overdrawing any account. The Python and JavaScript tests repeat the
fee, tax and several-account checks.
