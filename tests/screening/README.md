# S&P 500 screening data

The members of the S&P 500 with about five years of daily prices and a snapshot of their
fundamentals, for screening a whole index:

- `constituents.csv`: the members on the day the files were fetched, with each one's `symbol`,
  `name`, GICS `sector` and GICS sub-industry (`industry`). A company with two listed share
  classes, such as Alphabet, appears once for each.
- `prices.csv`: daily closes and volumes of each member, in the long layout `id,date,close,volume`
  that `Universe` reads, one row per security and trading day. Closes are adjusted for dividends
  and splits and given to 6 significant digits; ids are the list's symbols, such as `BRK.B`.
  A volume is empty where Yahoo Finance has none, and a day without a close is left out.
- `fields.csv`: one row per member, with its `id`, `name`, `sector`, `industry` and `exchange`,
  and its fundamentals on the day of the fetch: `market_cap` in US dollars, `trailing_pe` and
  `forward_pe` (the price over the last twelve months' earnings per share, and over the next
  year's expected), `price_to_book`, `dividend_yield` (the last twelve months' dividends over the
  price, as a fraction: 0.021 for 2.1%) and `eps_ttm` (earnings per share over the last twelve
  months). A value is empty where Yahoo Finance has none.

The members are today's, so the prices look back with today's knowledge: companies that left the
index in the five years are missing (survivorship bias), and the fundamentals are those of the
last day only. Results on this data test the engine; they are not evidence that a screen works.

The files are not part of the published packages. `scripts/fetch-screening-data.py` downloads
them: the members, sectors and sub-industries from the public-domain
[s-and-p-500-companies](https://github.com/datasets/s-and-p-500-companies) list, and the prices
and fundamentals from Yahoo Finance. It keeps Yahoo's answers in `.cache/`, which git ignores, so
a run that stops resumes where it stopped; delete the folder to fetch everything again. The prices
and fundamentals remain the property of their providers.
