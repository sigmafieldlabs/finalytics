# S&P 500 prices

Daily prices of 55 large US companies, used by the Rust, Python and JavaScript tests and by the
performance budgets to exercise the engine on a long daily history:

- `prices.csv`: daily adjusted closes, from 3 January 2005 to 30 October 2026, of 55 S&P 500
  companies: the five largest in each of the 11 GICS sectors by market value in November 2026,
  among members that traded throughout, with one share class per company. Adjusted closes
  include dividends and splits. A last column, `T-Bills`, is a return index of 3-month US
  Treasury bills: the 13-week rate, accrued each calendar day at actual/360.
- `index.csv`: the S&P 500 total return index on the same dates, a benchmark for the prices.
- `companies.csv`: each company's ticker, name, GICS sector and GICS sub-industry, in the order
  of the price columns.

The companies are chosen with hindsight: they are today's largest members, so their past returns
are higher than those of the index's members at the time (survivorship bias). Results on this
data test the engine; they are not evidence that a method works.

The files are not part of the published packages. `scripts/fetch-market-data.py` downloads them:
the prices, the total return index (`^SP500TR`) and the 13-week Treasury bill rate (`^IRX`) from
Yahoo Finance, and the members, sectors and sub-industries from the
[s-and-p-500-companies](https://github.com/datasets/s-and-p-500-companies) list. The prices and
index levels remain the property of their providers.
