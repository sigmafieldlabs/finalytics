# Tax lots and cost basis

Finalytics keeps a portfolio's books from its transactions. Every purchase opens a **tax lot**;
sales, transfers and corporate actions relieve or change lots; and each disposal records a
realised gain in the instrument's currency and in the base currency.

## Definitions

A lot holds units of one instrument in one account, acquired together:

| Field | Meaning |
| --- | --- |
| `quantity` | The units still held. |
| `cost_local` | The remaining cost in the instrument's currency, including acquisition fees. |
| `cost_base` | The same cost in the base currency, converted at the rate on each acquisition date. |
| `acquired` | The acquisition date, which sets the holding period. |

Lots keep **total** costs rather than unit costs, so splitting a lot can never create or lose
cost. When part of a lot is taken, its share of the cost is
$\text{cost} \times q_{\text{taken}} / q_{\text{held}}$, rounded to 10 decimal places, and the
lot keeps the rest; the two parts always add up to the cost before.

### Choosing lots for a sale

| Method | Lots relieved first |
| --- | --- |
| `fifo` (default) | The oldest |
| `lifo` | The newest |
| `highest_cost` | The highest cost per unit (defers gains) |
| `lowest_cost` | The lowest cost per unit |
| `average_cost` | Every lot in proportion, so each sale carries the average cost |
| `specific_id` | The lots the sale names, with a quantity for each |

Ties are broken by acquisition date, then by the order the lots were opened. An account sets its
default method; a sale may override it.

### Realised gains

A sale's proceeds are $q \times p \times m - \text{fees}$ (quantity, price, multiplier), shared
among the lots it closes in proportion to the units taken from each. For each lot:

$$\text{gain}_{\text{local}} = \text{proceeds}_{\text{local}} - \text{cost}_{\text{local}}$$

$$\text{gain}_{\text{base}} = \text{proceeds}_{\text{local}} \times x_{\text{disposal}} - \text{cost}_{\text{base}}$$

where $x_{\text{disposal}}$ is the rate on the disposal date. The base-currency gain therefore
includes the currency effect between acquisition and disposal.

### Corporate actions

| Action | Effect on the lots |
| --- | --- |
| Split (`from` → `to`) | Quantities scale by `to / from`; costs are unchanged. |
| Spin-off | Each lot gives a share `cost_allocation` of its cost to a new lot of the new instrument, with `ratio` new units per unit held, the same acquisition date, and the identifier `lot/transaction`. |
| Merger | Lots become lots of the new instrument with `ratio` new units per unit; cash paid per unit reduces the cost basis (any excess over the basis is realised). A merger for cash only is a sale. |
| Symbol change | Lots move to the new identifier unchanged (both must be priced in the same currency). |
| Return of capital | Reduces the cost basis of each lot in proportion to its units; any excess over the basis is realised as a gain. |

Dividends and interest are cash, net of tax withheld; they do not change lots.

## Conventions

- **Trade-date accounting.** Positions change on the trade date; the settlement date is recorded
  for cash reporting.
- **Fees.** Fees on a purchase are added to the cost; fees on a sale reduce the proceeds.
- **Corporate actions** apply to the account of the transaction that records them.
- **Transfers in** take the cost basis given, or the market value on the transfer date when none
  is given (converted to the base currency at that date's rate), and the acquisition date given,
  or the transfer date.
- **No short positions.** Selling or transferring out more than an account holds is an error.

## Example

```python
>>> import finalytics as fl
>>> data = fl.PortfolioData.from_csv(
...     instruments="id,kind,currency\nAAPL,equity,USD\n",
...     transactions=(
...         "id,account,trade_date,type,instrument,quantity,price,fees,amount,currency,lot_method\n"
...         "t1,main,2026-01-02,deposit,,,,,10000,USD,\n"
...         "t2,main,2026-01-05,buy,AAPL,10,150,5,,,\n"
...         "t3,main,2026-03-02,buy,AAPL,10,200,5,,,\n"
...         "t4,main,2026-06-01,sell,AAPL,12,190,6,,,highest_cost\n"
...     ),
...     base_currency="USD",
... )
>>> portfolio = fl.Portfolio(data)
>>> for gain in portfolio.realised_gains():
...     print(gain.lot, gain.quantity, gain.cost_local, gain.proceeds_local, gain.gain_local)
t3 10 2005 1895 -110
t2 2 301 379 78

```

The sale of 12 units at 190, less 6 of fees, raises 2,274. Highest cost first closes lot `t3`
(10 units costing 200.50 each) and takes 2 units of `t2` (150.50 each). What remains:

```python
>>> [(lot.id, lot.quantity, lot.cost_local) for lot in portfolio.holdings().lots]
[('t2', Decimal('8'), Decimal('1204'))]
>>> [(s.lot, s.quantity, s.cost_local) for s in portfolio.lots_for_sale("2026-06-30", "main", "AAPL", 5)]
[('t2', Decimal('5'), Decimal('752.50'))]

```

`lots_for_sale` previews which lots a sale would close, without changing the books.

## Validation

The ledger follows accounting rules rather than a model with a reference implementation, so it is
checked by worked examples and by properties:

- property tests (`crates/finalytics-ledger/src/lots.rs`) that relieving lots by any method, in
  any order, conserves both quantity and cost exactly;
- worked examples of the transaction types and corporate actions
  (`crates/finalytics-ledger/tests/books.rs`), with the expected lots, cash and gains computed by
  hand.

## References

- Internal Revenue Service, *Publication 550: Investment Income and Expenses*, on cost basis,
  identification of shares, and corporate actions.
- HM Revenue & Customs, *Capital Gains Manual*, on pooling and the identification of shares, for
  the average-cost method.
