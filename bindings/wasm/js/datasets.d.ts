
export declare namespace datasets {
  /**
   * Daily prices, from 3 January 2005 to 30 October 2026, of 55 S&P 500 companies: the five
   * largest in each of the 11 sectors, by market value in November 2026, among members that
   * traded throughout. The prices are adjusted closes, which include dividends and splits. A
   * last column, `T-Bills`, is the return index of 3-month US Treasury bills, for use as the
   * risk-free asset; `datasets.sp500Companies()` gives each company's sector. The companies are
   * chosen with hindsight, so their past returns are higher than those of the index's members at
   * the time (survivorship bias). The prices are a file of the package, read on first use.
   */
  function sp500(): Promise<PriceTable>;
  /**
   * The S&P 500 total return index, daily from 3 January 2005 to 30 October 2026, in one
   * column, `S&P 500`: a benchmark for `datasets.sp500()`. Read on first use.
   */
  function sp500Index(): Promise<PriceTable>;
}
