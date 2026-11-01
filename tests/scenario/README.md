# Shared scenario

A small multi-currency portfolio used by the Rust, Python and JavaScript tests, so that the same
inputs can be checked to give the same results in every language:

- `instruments.csv`: an equity fund, a European equity priced in euros, and a bond, with asset
  class and region classifications.
- `transactions.csv`: deposits into two accounts, purchases, a currency conversion, a dividend
  with tax withheld, a sale and a withdrawal.
- `prices.csv` and `fx.csv`: 44 business days of prices and EUR/USD rates, from 1 September to
  30 October 2026.

The prices are synthetic.

`expected.json` holds the results of the scenario, recorded by the Rust test
`crates/finalytics/tests/scenario.rs`: valuation, realised gains, returns, contributions,
statistics, tail risk, risk decomposition, optimisation and a rebalancing proposal. The Python
and JavaScript tests compute the same results and must match them: decimals exactly, and other
numbers to $10^{-12}$ relative. After a deliberate change to the engine, regenerate the file with

```sh
UPDATE_SCENARIO=1 cargo test -p finalytics --test scenario
```

and review the difference.
