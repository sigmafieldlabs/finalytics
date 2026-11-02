# Households

A household is several accounts seen together: the accounts of one person, or of a family, held
in different wrappers or with different providers. The household view reports the consolidated
performance, each account's own performance, the same for groups of accounts, and what each
account and each group contributed to the consolidated return.

## Accounts and their groups

Every account belongs to the portfolio, and its transactions name it. Accounts can carry
classifications by scheme, such as `owner: Alice` or `wrapper: ISA`, given in the portfolio
document or in an accounts file (columns `id`, `name`, `lot_method` and `class.<scheme>`). A
group is the accounts that share a classification; accounts without one are in the group
"unclassified". With an accounts file, a portfolio has exactly the accounts it lists, and a
transaction in any other account is an error.

## Values and flows by account

The books value each account at the end of each date, in the base currency, and record its
external flows: deposits and transfers in, positive; withdrawals and transfers out, negative;
each converted at the rate of its own date. Added up over every account, the values and flows
are the portfolio's. They are equal exactly unless a value needs more than the 28 significant
digits of decimal arithmetic, as one at a rate inverted from its quoted direction can: the
portfolio values each instrument's whole quantity once and each account values its own, so the
two can then differ by rounding in the last digits.

A group's values and flows are its accounts' added up. Money moved between two accounts, as a
withdrawal from one and a deposit into the other, is a flow for each account, and nets to zero
for any group that holds both, and for the household.

## Performance

Each account, each group and the household has its value at the start and end of the period,
its net flows, its gain (the change in value less the flows), its time-weighted return, its
money-weighted return, and its weight in the household at the start. The returns are measured on
its own values and flows exactly as for a portfolio (see [returns](returns.md)): daily returns
with flows at the start or the end of the day, chain-linked, and the internal rate of return of
the flows. An account whose time-weighted return cannot be measured, such as one that gained or
lost value on a day it started with nothing, reports none, with a warning.

## Contribution by account

An account's own return does not say how much it added to the household's: a small account with
a high return adds little. Each account's contribution is measured as a holding's is (see
[contribution](contribution.md)), on the household's base. With $g_{a,t}$ the account's gain on
day $t$ (the change in its value less its flows), $B_t$ the household's value invested at the
start of the day, and $G_t$ the household's growth up to day $t$,

$$C_a = \sum_t \frac{g_{a,t}}{B_t} \cdot \frac{G_{t-1}}{G_0}$$

Since the accounts' gains add up to the household's, $\sum_a g_{a,t} = B_t r_t$, the
contributions add up exactly to the household's time-weighted return, with no residual. A
group's contribution is its accounts' added up.

## Example

```python
>>> import finalytics as fl
>>> portfolio = fl.Portfolio.from_csv(
...     accounts=(
...         "id,name,class.owner,class.wrapper\n"
...         "main,Brokerage,Alice,taxable\n"
...         "isa,ISA,Alice,ISA\n"
...         "kid,Junior account,Bob,\n"
...     ),
...     instruments="id,kind,currency\nSPY,fund,USD\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,amount,currency\n"
...         "t1,main,2026-03-02,deposit,,,,10000,USD\n"
...         "t2,main,2026-03-02,buy,SPY,50,100,,\n"
...         "t3,isa,2026-03-02,deposit,,,,5000,USD\n"
...         "t4,isa,2026-03-02,buy,SPY,50,100,,\n"
...         "t5,kid,2026-03-03,deposit,,,,1000,USD\n"
...         "t6,main,2026-03-04,withdrawal,,,,2000,USD\n"
...         "t7,isa,2026-03-04,deposit,,,,2000,USD\n"
...     ),
...     base_currency="USD",
... )
>>> market = fl.MarketData(
...     prices={"SPY": {"2026-03-02": 100, "2026-03-03": 110, "2026-03-04": 99, "2026-03-05": 105}}
... )
>>> household = portfolio.books(market).household(group_by=["owner"])
>>> round(household.household.time_weighted, 6), household.household.flows
(0.03125, 1000.0)
>>> for a in household.accounts:
...     print(a.name, a.value_start, a.value_end, a.flows, round(a.time_weighted, 6), round(a.contribution, 6))
main 10000.0 8250.0 -2000.0 0.019118 0.015625
isa 5000.0 7250.0 2000.0 0.063333 0.015625
kid 0.0 1000.0 1000.0 0.0 0.0
>>> [(g.name, g.accounts, round(g.contribution, 6), g.flows) for g in household.groups[0].groups]
[('Alice', ['main', 'isa'], 0.03125, 0.0), ('Bob', ['kid'], 0.0, 1000.0)]

```

Alice moved 2,000 from her brokerage account to her ISA on the third day. Each account counts
the move as a flow, not as a gain or a loss. The ISA's 6.3% is higher than the brokerage
account's 1.9% because more of its value was in the fund, which rose 5% over the period. For
Alice's two accounts together, and for the household, the move is not a flow at all. Both
accounts held the same 50 units, so each contributed half of the household's 3.125%; Bob's
account held cash and contributed nothing.

## Validation

`scripts/reference/households.py` replays the transactions of the shared test portfolio account
by account in pandas, on every price date, including a currency conversion, a dividend, a sale
and a withdrawal, and computes each account's and each group's values, flows, time-weighted
return, money-weighted return (`scipy.optimize.brentq`) and contribution. The engine agrees to
$10^{-12}$ relative, and to $10^{-10}$ for money-weighted returns, the reference's root-finding
tolerance, in Rust, Python and JavaScript. The tests also work a three-account household by hand,
and check that the accounts' series add up to the portfolio's, exactly for the shared portfolio,
and that their contributions add up to the household's return.

## References

- CFA Institute, *Global Investment Performance Standards (GIPS) for Firms*, 2020: time-weighted
  returns and the treatment of external cash flows.
- Bacon, C. R., *Practical Portfolio Performance Measurement and Attribution*, 2nd ed., Wiley,
  2008.
