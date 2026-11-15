# JavaScript and TypeScript API reference

Generated from the npm package's TypeScript declarations and their JSDoc. Overloads
remain together, and the declaration text is that of this release; do not edit by hand.

## finalytics

<a id="finalytics-observed"></a>

### Observed

A price or rate found for a date.

```typescript
export interface Observed {
    /** The value. */
    value: Decimal;
    /** The date it was observed, earlier than the date asked for if it was carried forward. */
    date: Date;
    /** Whether it was carried forward from an earlier date. */
    carried: boolean;
}
```

<a id="finalytics-holdings"></a>

### Holdings

Open positions, tax lots and cash balances at a point in time.

```typescript
export interface Holdings {
    positions: Position[];
    lots: Lot[];
    cash: CashBalance[];
}
```

<a id="finalytics-document"></a>

### Document

A portfolio document: a portfolio's records and, optionally, its market data.

```typescript
export interface Document {
    portfolio: PortfolioData;
    market?: MarketData;
}
```

<a id="finalytics-rebalancechoices"></a>

### RebalanceChoices

Choices for `Portfolio.rebalance`, all optional.

```typescript
export interface RebalanceChoices {
    /** The accounts to rebalance together; every account by default. */
    accounts?: string[];
    /** The date of the prices used; the last price date by default. */
    as_of?: Date;
    /** The report's title, such as `"Morgan family portfolio"`. */
    name?: string;
    /** How sales choose lots without tax rates: each account's own method by default. */
    lot_method?: LotMethod;
    /** Trade whole units only, rounding each trade towards zero. */
    whole_units?: boolean;
    /** Leave out trades worth less than this in the base currency. */
    minimum_trade?: number;
    /** The fee on each trade, in basis points of its value, such as 5 for 0.05%. */
    fee_bps?: number;
    /** A fixed fee on each trade, in the instrument's currency. */
    fee_per_trade?: number;
    /** The smallest fee a trade pays, in the instrument's currency. */
    minimum_fee?: number;
    /**
     * Tax-aware rebalancing at these rates on short-term and long-term gains, such as
     * `[0.37, 0.2]`: sales in taxable accounts take the lots that cost the least tax.
     */
    tax?: [number, number];
    /** The holding period, in days, after which a gain is long-term: 365 by default. */
    long_term_after_days?: number;
    /**
     * `true` keeps every currency's cash as it is, and `["USD"]` that of the currencies named: the
     * other targets then share the rest of the value, so the trades neither invest that cash nor
     * add to it. Targets that leave the kept cash out are shares of the rest; targets that give it
     * its weight now, as an optimisation with the same `keep_cash` gives them, are shares of the
     * whole value. By default the targets are shares of the whole value, cash included.
     */
    keep_cash?: KeepCash;
}
```

<a id="finalytics-portfoliofiles"></a>

### PortfolioFiles

The CSV text of a portfolio's files, for `Portfolio.fromCsv`. Only the transactions are
required; `datasets.file("portfolio/transactions.csv")` and the other dataset files show each
layout, and the data guide describes every column:
https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md

```typescript
export interface PortfolioFiles {
    /**
     * One row per transaction: a `date` (or `trade_date`), a `type` and the columns the type
     * needs, such as `instrument`, `quantity` and `price` for a `buy`, or `amount` and `currency`
     * for a `deposit`. Rows may give an `id` (`t1`, `t2` and so on by default) and an `account`
     * (`main` by default).
     */
    transactions: string;
    /** A `date` column and one column per instrument, or `date`, `instrument` and `price`. */
    prices?: string;
    /** A `date` column and one column per currency pair, such as `EUR/USD`. */
    fx?: string;
    /**
     * `id`, `kind` and `currency`, with `class.<scheme>` columns such as `class.asset_class`.
     * Without it, each instrument is in the currency given with it in the transactions, or else in
     * the base currency.
     */
    instruments?: string;
    /** `id`, with `name`, `lot_method` and `class.<scheme>` columns. */
    accounts?: string;
    /**
     * Goals kept with the portfolio, one record per row: `goal` (its name) and `type`, a
     * `target`, `payments` or `projection` row with the plan's settings, and `saving`, `spending`,
     * `mix`, `alternative`, `glide` and `alternative_glide` rows.
     */
    goals?: string;
    /** The currency results are reported in; by default, that of the first deposit. */
    base_currency?: string;
    /** The field delimiter: a comma by default. */
    delimiter?: string;
}
```

<a id="finalytics-assetsoptions"></a>

### AssetsOptions

The settings of `Assets`, all optional.

```typescript
export interface AssetsOptions {
    /** The annual risk-free rate, for Sharpe ratios; 0 by default. */
    risk_free_rate?: number;
    /**
     * One of the assets, such as T-bills, as the risk-free asset instead: its mean return is the
     * risk-free rate, and optimisation leaves it out.
     */
    risk_free_asset?: string;
    /** 252 for daily returns, 52 for weekly, 12 for monthly; detected from the dates by default. */
    periods_per_year?: number;
    /**
     * `"ledoit_wolf"` (the default), `"sample"`, `"oas"`, `"ewma"` (decay 0.94),
     * `"constant_correlation"`, `"denoised"` or `"nonlinear_shrinkage"`, or a `CovarianceMethod`
     * such as `{ type: "statistical", factors: 3 }`.
     */
    covariance?: CovarianceMethod["type"] | CovarianceMethod;
    /**
     * How expected returns are estimated: `"historical"` (the default), `"james_stein"` (shrunk
     * towards the assets' average) or `"ewma"` (decay 0.94), or a `MeanMethod` such as
     * `{ type: "ewma", decay: 0.97 }`.
     */
    mean_returns?: MeanMethod["type"] | MeanMethod;
    /**
     * Each asset's class under classification schemes, such as
     * `{ asset_class: { "US Equity": "Equity", Bonds: "Fixed income" } }`, so that group limits can
     * name a class: `groups: { Equity: { classification: "asset_class", max: 0.6 } }`. A portfolio's
     * assets carry its instruments' classes.
     */
    classes?: Record<string, Record<string, string>>;
}
```

<a id="finalytics-reportchoices"></a>

### ReportChoices

Choices for `Portfolio.report`, all optional.

```typescript
export interface ReportChoices extends Partial<ReportOptions> {
    /** The portfolio's name, such as `"Morgan family portfolio"`, shown under the title. */
    name?: string;
}
```

<a id="finalytics-theme"></a>

### Theme

The colours of a report's page: `"auto"` (the default) follows the reader's light or dark
setting; `"light"` and `"dark"` keep to one.

```typescript
export type Theme = "auto" | "light" | "dark";
```

<a id="finalytics-reportmethods"></a>

### ReportMethods

The ways to lay out a report: as text, as an HTML page, or shown.

```typescript
export interface ReportMethods {
    /** The report as text for printing, 80 columns wide, with `name`, if given, under the title. */
    toText(name?: string): string;
    /**
     * The report as one self-contained HTML page, with `name`, if given, as its title: inline
     * styles and SVG charts, no scripts, in `theme` (the reader's light or dark setting by
     * default), ready to print.
     */
    toHtml(name?: string, theme?: Theme): string;
    /** In Node.js, writes `toHtml(name, theme)` to `path`, and returns the path. */
    saveHtml(path: string, name?: string, theme?: Theme): string;
    /**
     * Shows the report, in `theme`. In a browser, in a frame inside `element`, or else in a new
     * window. In Node.js, in the default browser, or with the command in the `BROWSER`
     * environment variable if it is set; the page is saved to a temporary file, whose path is
     * returned. Without `element`, a handler set with `setShowHandler` takes the page instead.
     */
    show(element?: Element, theme?: Theme): string | undefined;
}
```

<a id="finalytics-setshowhandler"></a>

### setShowHandler

Sets what `show()` does with a report's page when it is given no element: from then on it calls
`handler` with the page's HTML, instead of opening a window or a browser, and returns
`undefined`. `null` (or nothing) restores the default. For programs that show pages themselves,
such as an engine in a worker, which has no page of its own, or an app with a view for reports.
Anything else is refused with a `ValidationError`.

```typescript
export function setShowHandler(handler?: ((html: string) => void) | null): void;
```

<a id="finalytics-resultsmethods"></a>

### ResultsMethods

Raw results that print as text: `${results}` or `results.toText()`.

```typescript
export interface ResultsMethods {
    /** The results as text: the weights, or each strategy's periods, growth, trades and refits. */
    toText(): string;
    toString(): string;
}
```

<a id="finalytics-result"></a>

### Result

What `Assets.optimise`, `Assets.evaluate`, `Assets.compare`, `Assets.backtest`, `Assets.plan`,
`Portfolio.rebalance` and `Universe.screen` return: the report and the raw results.
`${result}` is the report as text, and `result.show()` shows it as a page.

```typescript
export interface Result<R, D> {
    /** The report. */
    report: R & ReportMethods;
    /** The raw results. */
    results: D & ResultsMethods;
    /** The report as text for printing, 80 columns wide, with `name`, if given, under the title. */
    toText(name?: string): string;
    /** The report as one self-contained HTML page, with `name`, if given, as its title. */
    toHtml(name?: string, theme?: Theme): string;
    /** In Node.js, writes the report's page to `path`, and returns the path. */
    saveHtml(path: string, name?: string, theme?: Theme): string;
    /** Shows the report, as `report.show` does. */
    show(element?: Element, theme?: Theme): string | undefined;
    toString(): string;
}
```

<a id="finalytics-optimiselimits"></a>

### OptimiseLimits

The limits on the weights and the methods' parameters, as for `Assets.optimise`; a backtest
trades from the weights it holds, so it takes no current weights.

```typescript
export type OptimiseLimits = Omit<Partial<OptimiseOptions>, "method" | "frontier" | "current_weights">;
```

<a id="finalytics-frontieroptions"></a>

### FrontierOptions

The options of `Assets.frontier`: the limits it is drawn under, as for `Assets.optimise`, the
expected returns, and the current and benchmark weights the limits start from. The risk-free
rate is the return of `Cash`, where cash limits hold it.

```typescript
export type FrontierOptions = Partial<
Pick<
OptimiseOptions,
| "bounds"
| "max_weight"
| "asset_bounds"
| "groups"
| "min_effective_assets"
| "cash"
| "expected_returns"
| "risk_free_rate"
| "current_weights"
| "max_turnover"
| "min_trade"
| "max_trades"
| "keep_cash"
| "benchmark_weights"
| "max_tracking_error"
>
>;
```

<a id="finalytics-optimisechoices"></a>

### OptimiseChoices

The options of `Assets.optimise`: those of `OptimiseOptions`, with the report's title.

```typescript
export interface OptimiseChoices extends Partial<OptimiseOptions> {
    /** The report's title, such as `"Defensive equity"`: the method's name by default. */
    name?: string;
}
```

<a id="finalytics-evaluatechoices"></a>

### EvaluateChoices

The options of `Assets.evaluate`: the weights, the estimates' options, and the frontier with
its limits, as for `Assets.optimise`.

```typescript
export interface EvaluateChoices
```

<a id="finalytics-validationchoice"></a>

### ValidationChoice

How strategies are validated: `"in_sample"` (the default); `{ holdout: { start: "2016-11-01" } }`,
fitted on the history before the date and tested on the rest; or
`{ walk_forward: { years: 5 } }`, refitted at each scheduled rebalance on the last five years
(or on all the history so far, with `expanding: true`).

```typescript
export type ValidationChoice =
| "in_sample"
| { holdout: { start: Date } }
```

<a id="finalytics-rebalancechoice"></a>

### RebalanceChoice

When the portfolio trades back to its targets: `"monthly"`, `"quarterly"` (the default),
`"yearly"`, `"never"`, `{ drift: { band: 0.05 } }` when a weight drifts five points away, or
`{ every: { periods: 21 } }` after every 21 periods.

```typescript
export type RebalanceChoice =
| "monthly"
| "quarterly"
| "yearly"
| "never"
| { drift: { band: number } }
```

<a id="finalytics-backtestsettings"></a>

### BacktestSettings

How `Assets.compare` and `Assets.backtest` run, all optional.

```typescript
export interface BacktestSettings {
    /** How the strategies are validated: in sample by default. */
    validation?: ValidationChoice;
    /** When the portfolio trades back to its targets: quarterly by default. */
    rebalance?: RebalanceChoice;
    /** The cost of trading, in basis points of the value traded: 10 is 0.1%. */
    trading_cost_bps?: number;
    /**
     * What the strategies are compared with: weights by asset, such as
     * `{ "US Equity": 0.6, Bonds: 0.4 }`, or an index's prices, as a `PriceTable` of one column.
     */
    benchmark?: Record<string, number> | PriceTable | Benchmark;
    /** The benchmark's name: `"Benchmark"` by default, or the prices' column name. */
    benchmark_name?: string;
    /** The value each run starts with, to follow money: the reports then give each run's ending value and money-weighted return. */
    start_value?: number;
    /** Money paid in (positive) or taken out (negative) on dates, such as `[{ date: "2020-01-31", amount: 500 }]`. */
    flows?: CashFlow[];
}
```

<a id="finalytics-comparechoices"></a>

### CompareChoices

Choices for `Assets.compare`: the methods, with limits and settings applied to each.

```typescript
export interface CompareChoices extends OptimiseLimits, BacktestSettings {
    /**
     * The methods to compare, such as `["equal_weight", "minimum_variance"]`, in the order of the
     * results and the report. Fixed weights may come among them, as
     * `{ name: "60/40", weights: { "US Equity": 0.6, Bonds: 0.4 } }`.
     */
    methods?: (Method | { name: string; weights: Record<string, number> })[];
    /**
     * Fixed weights by name, such as `{ "60/40": { "US Equity": 0.6, Bonds: 0.4 } }`, held and
     * rebalanced as the methods are and scored on the same periods; not fitted to any returns.
     * They come after `methods`, in the order of the object's keys.
     */
    weights?: Record<string, Record<string, number>>;
    /** The report's title, such as `"S&P 500 stocks"`. */
    name?: string;
}
```

<a id="finalytics-backtestchoices"></a>

### BacktestChoices

Choices for `Assets.backtest`: a method, with its limits, or fixed weights.

```typescript
export type BacktestChoices =
| (OptimiseLimits &
BacktestSettings & {
    /** The method, refitted as the validation says. */
    method: Method;
```

<a id="finalytics-mixchoice"></a>

### MixChoice

An investment mix: weights by asset, adding up to one, or a `Mix`, such as a glide path.

```typescript
export type MixChoice = Record<string, number> | Mix;
```

<a id="finalytics-planchoices"></a>

### PlanChoices

Choices for `Assets.plan`: those of `PlanOptions`, with plainer forms for the mixes, the
inflation, the chance and the model.

```typescript
export interface PlanChoices
```

<a id="finalytics-scenariochoices"></a>

### ScenarioChoices

Choices for `Portfolio.shockScenario` and `Portfolio.historicalScenario`, all optional.

```typescript
export interface ScenarioChoices extends Partial<ScenarioOptions> {
    /** The date of the holdings; the last price date by default. */
    date?: Date;
}
```

<a id="finalytics-seriesinput"></a>

### SeriesInput

One security's prices for `new Universe`, on its own dates.

```typescript
export interface SeriesInput {
    /** The security's identifier, such as `"AAPL"`. */
    id: string;
    /** The dates of its closes, in increasing order: ISO strings, or days since 1970-01-01. */
    dates: Date[] | Int32Array;
    /** Its close on each date; NaN or `null` is no close, and its date is left out. */
    close: Float64Array | (number | null)[];
    /** The volume traded on each date, if known. */
    volume?: Float64Array | (number | null)[];
    /** The dividend per share paid on each date, zero on the others, if known. */
    dividends?: Float64Array | (number | null)[];
    /** Values the library does not compute, such as `{ sector: "Technology", pe_ratio: 18.4 }`. */
    fields?: Record<string, FieldValue>;
}
```

<a id="finalytics-universecolumns"></a>

### UniverseColumns

Many securities' prices as flat columns, for `Universe.fromColumns`: every security's rows one
after another, so that thousands of histories cross as a few typed arrays.

```typescript
export interface UniverseColumns {
    /** Each security's identifier. */
    ids: string[];
    /** The row each security starts at: the first is 0. */
    starts: Int32Array | number[];
    /** Each row's date: days since 1970-01-01, or ISO strings. */
    dates: Int32Array | Date[];
    /** Each row's close; NaN where there is none. */
    close: Float64Array;
    /** Each row's volume, if known; NaN where not. */
    volume?: Float64Array;
    /** Each row's dividend per share, if known: zero where none is paid. */
    dividends?: Float64Array;
    /** Each field's values in the order of `ids`: numbers (NaN or `null` where missing) or text. */
    fields?: Record<string, Float64Array | (number | null)[] | (string | null)[]>;
}
```

<a id="finalytics-universesettings"></a>

### UniverseSettings

The settings of a `Universe`, all optional: those of `Universe.fromColumns`.

```typescript
export interface UniverseSettings {
    /** One of the securities, such as an index, for beta, correlation and relative returns. */
    benchmark?: string;
    /** The annual risk-free rate of the Sharpe and Sortino ratios; 0 by default. */
    risk_free_rate?: number;
}
```

<a id="finalytics-universeoptions"></a>

### UniverseOptions

The options of `new Universe` and `Universe.fromCsv`, all optional.

```typescript
export interface UniverseOptions extends UniverseSettings {
    /**
     * Values the library does not compute, such as a sector or a price-to-earnings ratio: CSV text
     * with an `id` column and one column per field, or values by security and then by name, such as
     * `{ AAPL: { sector: "Technology", pe_ratio: 31.2 } }`.
     */
    fields?: string | Record<string, Record<string, FieldValue>>;
}
```

<a id="finalytics-screenchoices"></a>

### ScreenChoices

The options of `Universe.screen`: those of `ScreenOptions`, with the report's title.

```typescript
export interface ScreenChoices extends Partial<ScreenOptions> {
    /** The report's title, such as `"Quality at a fair price"`. */
    name?: string;
}
```

<a id="finalytics-screencolumns"></a>

### ScreenColumns

The raw results of `Universe.screen`, as columns in the order of `ids`: one `Float64Array` per
measure, NaN where a security has no value, and one array per field.

```typescript
export interface ScreenColumns {
    /** The date measured to. */
    as_of: Date;
    /** The most days a last close could be before `as_of` without being stale. */
    max_carry_days: number;
    /** The benchmark, if any. */
    benchmark?: string | null;
    /** The annual risk-free rate of the Sharpe and Sortino ratios. */
    risk_free_rate: number;
    /** The securities, in the universe's order. */
    ids: string[];
    /** Whether each was measured. */
    status: ScreenStatus[];
    /** Whether each meets every rule. */
    passed: boolean[];
    /** Whether each is one of the matches kept. */
    matched: boolean[];
    /** Each one's place among the matches, from 1; NaN for the others. */
    rank: Float64Array;
    /** The date of each one's first close. */
    first_date: (Date | null)[];
    /** The date of each one's last close on or before `as_of`. */
    last_date: (Date | null)[];
    /** Each measure computed, in order, with every security's value. */
    measures: Partial<Record<Measure, Float64Array>>;
    /** Each field given, then each formula, with every security's value: numbers or text. */
    fields: Record<string, Float64Array | (string | null)[]>;
    /** The formulas, in order, with how many securities have no value of each. */
    formulas: FormulaResult[];
    /** Each rule, with how many met it. */
    rules: RuleResult[];
    /** How the matches are sorted. */
    sort?: Sort | null;
    /** The most matches kept. */
    limit?: number | null;
    /** The matches kept, in order. */
    matches: string[];
    /** How many securities were measured, passed and matched. */
    coverage: Coverage;
    /** Problems that did not stop the screen. */
    warnings: ValidationReport;
}
```

<a id="finalytics-validationerror"></a>

### ValidationError

A validation error: every problem found, with where it is.

```typescript
export interface ValidationError extends Error {
    name: "ValidationError";
    issues: Issue[];
}
```

<a id="finalytics-account"></a>

### Account

An account in a portfolio, such as a brokerage or custody account.

```typescript
export interface Account {
    /** The caller's identifier. */
    id: AccountId;
    /** A display name. */
    name?: string | null;
    /** How sales in this account choose lots, unless a sale says otherwise. */
    lot_method?: LotMethod;
    /**
     * Classifications by scheme, for grouping accounts: for example `owner: Alice` or
     * `wrapper: ISA`.
     */
    classifications?: Record<string, string>;
    /**
     * Whether gains are taxed when they are realised: true by default. A retirement account,
     * or another tax-deferred or tax-exempt account, is not taxable, so rebalancing estimates
     * no tax on its sales.
     */
    taxable?: boolean;
}
```

<a id="finalytics-accountholdings"></a>

### AccountHoldings

One account's holdings.

```typescript
export interface AccountHoldings {
    /** The account. */
    account: AccountId;
    /** The account's name, if it has one. */
    name?: string | null;
    /** The account's classifications. */
    classifications: Record<string, string>;
    /** The account's value in the base currency. */
    value_base: Decimal;
    /** The share of the portfolio's value. */
    weight: number;
    /** Its positions. */
    positions: PositionValue[];
    /** Its cash balances. */
    cash: CashValue[];
}
```

<a id="finalytics-accountid"></a>

### AccountId

Identifies an account within a portfolio.

Identifiers are non-empty, have no leading or trailing whitespace, and are compared
exactly (case matters).

```typescript
export type AccountId = string;
```

<a id="finalytics-accountline"></a>

### AccountLine

One account's values and external flows on each date of an `AccountSeries`.

```typescript
export interface AccountLine {
    /** The account. */
    account: AccountId;
    /** The account's value at the end of each date, in the base currency. */
    values: Decimal[];
    /**
     * The external flows into the account since the previous date (deposits and transfers in
     * positive; withdrawals and transfers out negative), each converted at its own date's rate.
     */
    flows: Decimal[];
}
```

<a id="finalytics-accountseries"></a>

### AccountSeries

Each account's values and external flows on a set of dates: the input to performance by
account and by group of accounts.

```typescript
export interface AccountSeries {
    /** The dates. */
    dates: Date[];
    /** One line per account, in the portfolio's order. */
    lines: AccountLine[];
    /** Prices and rates carried forward from earlier dates. */
    warnings: ValidationReport;
}
```

<a id="finalytics-accounttrades"></a>

### AccountTrades

One account's part of a rebalance.

```typescript
export interface AccountTrades {
    /** The account's identifier. */
    account: string;
    /** The account's name, or its identifier. */
    name: string;
    /** Whether its gains are taxed when realised. */
    taxable: boolean;
    /** Its value before the trades. */
    value_base: number;
    /** The one-way turnover: half the changes in each holding's weight, cash included. */
    turnover: number;
    /** Sales, then purchases. */
    trades: TradeRow[];
    /** Currency conversions. */
    conversions: ConversionRow[];
    /** The fees of its trades. */
    fees_base: number;
    /** The gain its sales realise, after fees. */
    realised_gain_base: number;
    /** With tax rates, the part of that gain from lots held long-term. */
    long_term_gain_base?: number | null;
    /** With tax rates and a taxable account, the estimated tax on its sales. */
    tax_base?: number | null;
    /**
     * What to check before making the trades: carried prices, a shortfall of cash, and trades
     * left out.
     */
    warnings: Issue[];
}
```

<a id="finalytics-accountsperformance"></a>

### AccountsPerformance

The performance of one account, a group of accounts or the whole household over a period.

```typescript
export interface AccountsPerformance {
    /** The account's identifier, the group's name, or "household". */
    name: string;
    /** The accounts it includes. */
    accounts: AccountId[];
    /** The value at the start of the period, in the base currency. */
    value_start: number;
    /** The value at the end of the period. */
    value_end: number;
    /** The external flows over the period, net: positive when more came in than went out. */
    flows: number;
    /** `value_end - value_start - flows`. */
    gain: number;
    /**
     * The share of the household's value at the start, absent if the household started with
     * nothing.
     */
    weight_start?: number | null;
    /**
     * The time-weighted return over the period, absent when it cannot be measured, such as for
     * an account that gained or lost value on a day it started with none.
     */
    time_weighted?: number | null;
    /**
     * The money-weighted return (annualised internal rate of return), absent when it cannot be
     * found.
     */
    money_weighted?: number | null;
    /**
     * The contribution to the household's time-weighted return: each day's gain over the
     * household's starting value for the day, linked by the household's growth. The
     * contributions of all accounts add up to the household's return.
     */
    contribution: number;
}
```

<a id="finalytics-allocation"></a>

### Allocation

How a transaction moved value between holdings, used to measure each holding's
contribution. Value moved between holdings nets to zero; only external flows and expenses
change the portfolio's total.

```typescript
export type Allocation =
| {
    type: "trade";
```

<a id="finalytics-allocationrow"></a>

### AllocationRow

One asset in the weights.

```typescript
export interface AllocationRow {
    /** The asset. */
    asset: string;
    /** Its weight. */
    weight: number;
    /** Its annual expected return. */
    expected_return: number;
    /** Its annual volatility. */
    volatility: number;
    /**
     * Its share of the portfolio's risk: of its variance (its weight times its covariance with
     * the portfolio, over the portfolio's variance), or for minimum CVaR of its expected
     * shortfall (its weight times its mean loss in the worst periods, over the expected
     * shortfall).
     */
    risk_share: number;
}
```

<a id="finalytics-amount"></a>

### Amount

A figure as a fraction and as an amount of money, such as a value at risk.

```typescript
export interface Amount {
    /** As a fraction of the portfolio's value. */
    fraction: number;
    /** As an amount in the base currency. */
    value: number;
}
```

<a id="finalytics-amountchange"></a>

### AmountChange

A change of an amount that reaches, or keeps, the chance wanted.

```typescript
export interface AmountChange {
    /** What is changed, such as "Monthly saving". */
    what: string;
    /**
     * The flow changed, by its place in the plan's flows; absent for the start value or the
     * goal's amount.
     */
    flow?: number | null;
    /** The amount in the plan. */
    from: number;
    /** The amount that gives the chance wanted, the rest of the plan as it is. */
    to: number;
}
```

<a id="finalytics-assetbound"></a>

### AssetBound

Limits on one asset's weight, in place of the default bounds.

```typescript
export interface AssetBound {
    /** The asset's label. */
    asset: string;
    /** The smallest weight allowed. */
    min: number;
    /** The largest weight allowed. */
    max: number;
}
```

<a id="finalytics-assetfit"></a>

### AssetFit

How well a factor model explains one asset's returns over the estimation period.

```typescript
export interface AssetFit {
    /** The asset's label. */
    asset: string;
    /**
     * The intercept: the regression's alpha, or the mean return for a statistical model;
     * absent for a fundamental model.
     */
    intercept?: number | null;
    /**
     * The share of the asset's return variance the factors explain. Absent when the asset's
     * returns do not vary.
     */
    r_squared?: number | null;
}
```

<a id="finalytics-assetgroup"></a>

### AssetGroup

A group of assets whose total weight is limited: the assets listed, or those classed as the
group's name under a classification, such as the group "Equity" under `asset_class`.

```typescript
export interface AssetGroup {
    /** The assets in the group; or give `classification`. */
    assets?: string[];
    /**
     * A classification scheme of the assets, such as `asset_class`: the group is the assets
     * classed as its name under it.
     */
    classification?: string | null;
    /** The smallest total weight: 0 by default. */
    min?: number;
    /** The largest total weight: 1 by default. */
    max?: number;
}
```

<a id="finalytics-assetstatistics"></a>

### AssetStatistics

One asset's statistics, with its value at risk and expected shortfall at 95% (historical).

```typescript
export interface AssetStatistics {
    /** The asset. */
    asset: string;
    /** Its statistics, annualised with the assets' periods per year and risk-free rate. */
    statistics: Statistics;
    /** Its value at risk and expected shortfall over one period at 95%, from history. */
    tail_risk: TailRisk;
}
```

<a id="finalytics-attribution"></a>

### Attribution

Brinson–Fachler attribution over one or more periods.

```typescript
export interface Attribution {
    /** How the periods were linked. */
    linking: Linking;
    /** The portfolio's return over all the periods, compounded. */
    portfolio_return: number;
    /** The benchmark's return over all the periods, compounded. */
    benchmark_return: number;
    /** `portfolio_return - benchmark_return`, which the linked effects add up to. */
    excess_return: number;
    /**
     * Each segment's effects over all the periods, linked, in the order the segments first
     * appear.
     */
    segments: SegmentEffects[];
    /** The allocation effects of all the segments, linked. */
    allocation: number;
    /** The selection effects of all the segments, linked. */
    selection: number;
    /** The interaction effects of all the segments, linked. */
    interaction: number;
    /** Each period's own attribution, before linking. */
    periods: PeriodAttribution[];
}
```

<a id="finalytics-attributioninputs"></a>

### AttributionInputs

What the attribution report compares the portfolio with.

```typescript
export interface AttributionInputs {
    /**
     * The instrument classification scheme that defines the segments, such as `asset_class`.
     * Cash is the segment "cash", and instruments without the classification "unclassified".
     */
    scheme: string;
    /**
     * The benchmark's segments in each period, such as each month. Without them, they are
     * derived from a blended benchmark: one period for each calendar month of the report, each
     * segment's weight the sum of its holdings' weights and its return their weighted return
     * over the month, by the classification `scheme` of the portfolio's instruments. A period
     * that reaches past the report's start or date is clipped to it, with a warning, and the
     * benchmark's figures are used as given; one wholly outside the report is left out.
     */
    periods?: BenchmarkPeriod[];
    /** How the periods are linked: Cariño's method by default. */
    linking?: Linking;
}
```

<a id="finalytics-attributionperiod"></a>

### AttributionPeriod

The segments over one period. The portfolio weights add up to one, as do the benchmark
weights.

```typescript
export interface AttributionPeriod {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /** The segments. */
    segments: Segment[];
}
```

<a id="finalytics-attributionreport"></a>

### AttributionReport

The attribution report.

```typescript
export interface AttributionReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The classification scheme of the segments. */
    scheme: string;
    /**
     * Each period's segments: the portfolio's weights and returns from its books, beside the
     * benchmark's.
     */
    periods: AttributionPeriod[];
    /**
     * The attribution, by period and linked: the totals for a waterfall chart, and each
     * period's effects for a chart over time.
     */
    attribution: Attribution;
    /**
     * The portfolio's time-weighted return over the same span, for comparison with the
     * attribution's portfolio return (`sum w r` in each period, linked), which leaves out
     * trading within each period.
     */
    time_weighted?: number | null;
}
```

<a id="finalytics-backtest"></a>

### Backtest

A backtest: the backtest report on one allocation against its benchmark, and the raw
results. Printing it prints the report, and showing it shows the report as a page.

```typescript
export interface Backtest {
    /** The backtest report. */
    report: BacktestReport;
    /** The raw results: every return and trade, in and out of sample. */
    results: BacktestResults;
}
```

<a id="finalytics-backtestcontext"></a>

### BacktestContext

What a backtest report describes: the data, how the strategies were tested, and the digest
of the inputs.

```typescript
export interface BacktestContext {
    /** The engine's version. */
    version: string;
    /**
     * A SHA-256 digest of every input, in hexadecimal: the assets' returns and the cash's,
     * the strategies with their weights and options, the benchmark, the validation and the
     * money. The same inputs give the same report, and different inputs a different digest;
     * the title, which printing can also give, is not one of them.
     */
    digest: string;
    /** The assets the strategies hold, other than the risk-free asset. */
    assets: string[];
    /** The risk-free asset, held as cash, if one of the assets is. */
    risk_free_asset?: string | null;
    /** The number of periods in a year. */
    periods_per_year: number;
    /** The covariance estimator of the optimisations. */
    covariance: CovarianceMethod;
    /** The estimator of the optimisations' mean returns. */
    mean_returns?: MeanMethod;
    /** The limits on the weights, in words. */
    limits: string[];
    /** How the backtest was run. */
    settings: Settings;
    /** The date the first period of the data ends on. */
    start: Date;
    /** The date the last period of the data ends on. */
    end: Date;
    /** The first period the strategies are scored on. */
    test_start: Date;
    /** The last period the strategies are scored on. */
    test_end: Date;
    /** For a holdout, the last period the weights were fitted to. */
    fit_end?: Date | null;
    /** For walk-forward validation, the periods of history before the first fit. */
    window?: number | null;
    /** The annual risk-free return over the test, compounded from the periods' returns. */
    risk_free_rate: number;
    /** The date the first period of the data starts from, when it is known. */
    origin?: Date | null;
    /** The value each run starts with, when the backtest follows money. */
    start_value?: number | null;
    /** Money paid in or taken out on dates, when the backtest follows money. */
    flows?: CashFlow[];
}
```

<a id="finalytics-backtestoptions"></a>

### BacktestOptions

What a backtest tests and how.

```typescript
export interface BacktestOptions {
    /** The strategies, in the order of the results. */
    strategies?: Strategy[];
    /** How the strategies are validated: in sample by default. */
    validation?: Validation;
    /** When the portfolio trades back to its targets: quarterly by default. */
    rebalance?: RebalanceSchedule;
    /** The cost of trading, in basis points of the value traded: 10 is 0.1%. */
    trading_cost_bps?: number;
    /** What the strategies are compared with, if anything. */
    benchmark?: Benchmark | null;
    /**
     * The value each run starts with, to follow money as well as returns: the reports then
     * give each run's ending value and money-weighted return. Zero, with flows, if absent.
     */
    start_value?: number | null;
    /**
     * Money paid in (positive) or taken out (negative) on dates, at the end of the period each
     * falls in, after its return.
     */
    flows?: CashFlow[];
}
```

<a id="finalytics-backtestreport"></a>

### BacktestReport

One allocation, tested against its benchmark: the backtest report.

```typescript
export interface BacktestReport {
    /** What was tested and how. */
    context: BacktestContext;
    /** The allocation. */
    strategy: StrategyReport;
    /** The benchmark, if one was given, held alike. */
    benchmark?: StrategyReport | null;
    /** For a holdout, both periods together, in sample then out of sample. */
    history?: History | null;
    /**
     * The weights held in each period of the test, by asset: the largest few, then the rest
     * together, then cash; at the same dates as the growth.
     */
    held: Series[];
    /**
     * The report's title, such as "Maximum Sharpe, fitted to 2016": shown when it is printed
     * or laid out as a page.
     */
    name?: string | null;
}
```

<a id="finalytics-backtestresults"></a>

### BacktestResults

The result of a backtest: every strategy, and the benchmark if one was given, on the same
assets and dates.

```typescript
export interface BacktestResults {
    /** The assets the strategies hold. */
    assets: string[];
    /** The date each period of the data ends on. */
    dates: Date[];
    /** The return of cash in each period: the risk-free return. */
    cash: number[];
    /** The number of periods in a year. */
    periods_per_year: number;
    /** How the backtest was run. */
    settings: Settings;
    /** Each strategy, in the order given. */
    strategies: StrategyResult[];
    /** The benchmark, if one was given. */
    benchmark?: StrategyResult | null;
}
```

<a id="finalytics-benchmark"></a>

### Benchmark

What the strategies are compared with.

```typescript
export type Benchmark =
| {
    type: "weights";
```

<a id="finalytics-benchmarkperiod"></a>

### BenchmarkPeriod

The benchmark's segments over one period.

```typescript
export interface BenchmarkPeriod {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /** The segments, with weights adding up to one. */
    segments: BenchmarkSegment[];
}
```

<a id="finalytics-benchmarksegment"></a>

### BenchmarkSegment

One segment of the benchmark over one period.

```typescript
export interface BenchmarkSegment {
    /** The segment: a classification of the scheme, such as `Equity`. */
    name: string;
    /** The benchmark's weight in the segment at the start of the period. */
    benchmark_weight: number;
    /** The benchmark's return in the segment over the period. */
    benchmark_return: number;
}
```

<a id="finalytics-bin"></a>

### Bin

One bin of a `Histogram`.

```typescript
export interface Bin {
    /** The lower edge, included. */
    lower: number;
    /** The upper edge, excluded (included for the last bin). */
    upper: number;
    /** The number of observations in the bin. */
    count: number;
    /** The share of all observations in the bin. */
    share: number;
    /**
     * The share a normal distribution with the observations' mean and standard deviation puts
     * in the bin, for comparison.
     */
    normal_share: number;
}
```

<a id="finalytics-blacklitterman"></a>

### BlackLitterman

The inputs to the Black–Litterman model.

```typescript
export interface BlackLitterman {
    /**
     * The covariance matrix of the assets' returns, in the units of the returns wanted, for
     * example annualised.
     */
    covariance: Matrix;
    /** The prior expected returns. */
    prior: Prior;
    /** The views; without any, the posterior returns are the prior. */
    views?: View[];
    /**
     * The uncertainty of the prior relative to the covariance, greater than zero: 0.05 by
     * default.
     */
    tau?: number;
}
```

<a id="finalytics-blacklittermanestimate"></a>

### BlackLittermanEstimate

The posterior of the Black–Litterman model.

```typescript
export interface BlackLittermanEstimate {
    /** The asset labels, in the order of the covariance matrix. */
    labels: string[];
    /** The prior expected returns. */
    prior_returns: number[];
    /** The posterior expected returns, which blend the prior with the views. */
    posterior_returns: number[];
    /**
     * The covariance of returns given the views, `C + M`, where `M` is the posterior uncertainty
     * of the expected returns (He and Litterman, 2002).
     */
    posterior_covariance: Matrix;
    /** The variance of each view, in the order of the views. */
    view_variances: number[];
}
```

<a id="finalytics-bound"></a>

### Bound

Lower and upper limits on a weight.

```typescript
export interface Bound {
    /** The smallest weight allowed; negative for a short position. */
    min: number;
    /** The largest weight allowed. */
    max: number;
}
```

<a id="finalytics-breakdown"></a>

### Breakdown

A whole split into parts by one dimension, such as by currency or by asset class.

```typescript
export interface Breakdown {
    /** The dimension: "holding", "currency", "kind", "account", or a classification scheme. */
    by: string;
    /** The parts, largest first. */
    slices: Slice[];
}
```

<a id="finalytics-calendarreturn"></a>

### CalendarReturn

The return over one calendar year, or the part of it in the run.

```typescript
export interface CalendarReturn {
    /** The year. */
    year: number;
    /** The date the year's first period ends on. */
    start: Date;
    /** The date the year's last period ends on. */
    end: Date;
    /** Whether the run covers the whole year. */
    complete: boolean;
    /** The compounded return. */
    value: number;
}
```

<a id="finalytics-cashbalance"></a>

### CashBalance

A cash balance in one account and currency.

```typescript
export interface CashBalance {
    /** The account. */
    account: AccountId;
    /** The currency. */
    currency: Currency;
    /** The balance, negative if overdrawn. */
    amount: Decimal;
}
```

<a id="finalytics-cashflow"></a>

### CashFlow

Money paid into a backtest's runs (a positive amount) or taken out (a negative one) on a date.

```typescript
export interface CashFlow {
    /** The date: the money moves at the end of the period it falls in. */
    date: Date;
    /** The amount: positive paid in, negative taken out. */
    amount: number;
}
```

<a id="finalytics-cashvalue"></a>

### CashValue

A cash balance valued on a date.

```typescript
export interface CashValue {
    /** The account. */
    account: AccountId;
    /** The currency. */
    currency: Currency;
    /** The balance. */
    amount: Decimal;
    /** Units of base currency per unit of `currency`. */
    fx_rate: Decimal;
    /** The balance in the base currency. */
    value_base: Decimal;
}
```

<a id="finalytics-cell"></a>

### Cell

One cell of a `ReportTable`: a number, text, yes or no, or nothing.

```typescript
export type Cell = number | boolean | string | null;
```

<a id="finalytics-changes"></a>

### Changes

What would change the outcome, one change at a time on the same simulations: below the
chance wanted, the smallest change that reaches it; at or above it, the largest change the
plan could absorb and keep it. A change that no amount within reach achieves is absent.

```typescript
export interface Changes {
    /** Whether the changes reach the chance wanted (true) or keep it (false). */
    to_reach: boolean;
    /** The plan's largest regular saving. */
    saving?: AmountChange | null;
    /** The value at the start. */
    start_value?: AmountChange | null;
    /** The goal's amount: the target, or each payment. */
    amount?: AmountChange | null;
    /** The target's date, or the month of the first payment, saving until then. */
    date?: MonthChange | null;
}
```

<a id="finalytics-column"></a>

### Column

One column of a `ReportTable`.

```typescript
export interface Column {
    /** A stable key for the column, such as `value_base`. */
    key: string;
    /** A heading for people, such as "Value (USD)". */
    label: string;
    /** What it holds. */
    kind: ColumnKind;
}
```

<a id="finalytics-columnkind"></a>

### ColumnKind

What a column holds, so that an export can format it.

```typescript
export type ColumnKind =
| {
    type: "text";
```

<a id="finalytics-comparison"></a>

### Comparison

A comparison: the comparison report on several methods backtested side by side, and the
raw results. Printing it prints the report, and showing it shows the report as a page.

```typescript
export interface Comparison {
    /** The comparison report: each method scored on the periods it was tested on. */
    report: ComparisonReport;
    /** The raw results: every return and trade of each method, in and out of sample. */
    results: BacktestResults;
}
```

<a id="finalytics-comparisonreport"></a>

### ComparisonReport

Methods tested the same way, side by side: the comparison report.

```typescript
export interface ComparisonReport {
    /** What was tested and how. */
    context: BacktestContext;
    /** Each method, in the order given. */
    strategies: StrategyReport[];
    /** The benchmark, if one was given, tested alike. */
    benchmark?: StrategyReport | null;
    /**
     * The report's title, such as "S&P 500 stocks": shown when it is printed or laid out as a
     * page.
     */
    name?: string | null;
}
```

<a id="finalytics-concentration"></a>

### Concentration

How concentrated the portfolio's instruments are. Every share is of the value invested in
instruments, cash aside, so that the figures agree with each other.

```typescript
export interface Concentration {
    /** The largest instrument holdings, largest first, with their share of the value invested. */
    largest: Slice[];
    /** The share of the value invested in the five largest instruments. */
    top_five: number;
    /** The share of the value invested in the ten largest instruments. */
    top_ten: number;
    /**
     * The Herfindahl–Hirschman index of the instruments' shares of the value invested in
     * instruments, from 1/n (equal) to 1 (one instrument).
     */
    herfindahl?: number | null;
    /** `1 / herfindahl`: the number of equal holdings that would be as concentrated. */
    effective_holdings?: number | null;
}
```

<a id="finalytics-constraints"></a>

### Constraints

The constraints on a portfolio. The default is fully invested and long-only.

```typescript
export interface Constraints {
    /** The sum of the weights; 1 by default. */
    budget?: number;
    /** Bounds on every weight; `[0, 1]` by default. */
    bounds?: Bound;
    /** Bounds on particular assets, in place of `bounds`. */
    asset_bounds?: AssetBound[];
    /** Limits on groups of assets. */
    groups?: GroupLimit[];
    /** The largest gross exposure, `sum |w|`, for portfolios with short positions. */
    leverage?: number | null;
    /** A turnover limit and trading costs. */
    turnover?: Turnover | null;
    /** A tracking-error limit. */
    tracking_error?: TrackingErrorLimit | null;
    /** Limits on factor exposures; they need a factor model. */
    factor_exposures?: FactorExposureLimit[];
    /** Limits on the size and number of trades; they need the turnover's current weights. */
    trades?: TradeLimits | null;
    /**
     * The smallest expected return, net of trading costs, in the units of the expected
     * returns: a floor for the objectives that do not use expected returns, such as minimum
     * CVaR.
     */
    min_return?: number | null;
    /**
     * The smallest effective number of assets, `1 / sum w_i^2`, from 1 to the number of
     * assets: the constraint `sum w_i^2 <= 1 / k` of DeMiguel, Garlappi, Nogales and Uppal
     * (2009), which keeps the weights from concentrating in a few assets.
     */
    min_effective_assets?: number | null;
}
```

<a id="finalytics-contribution"></a>

### Contribution

What one holding added to the portfolio's return over a period.

```typescript
export interface Contribution {
    /** The holding. */
    holding: HoldingKey;
    /**
     * Its contribution to the portfolio's time-weighted return. Contributions of all holdings,
     * with what fees and taxes charged to the accounts took (see `Performance::expenses`), add
     * up to the portfolio's return over the period.
     */
    contribution: number;
    /** The holding's own return in the base currency, when it can be measured. */
    return_base?: number | null;
    /** The holding's own return in its own currency, when it can be measured. */
    return_local?: number | null;
    /** `return_base - return_local`: what currency movements added or took away. */
    currency_effect?: number | null;
    /** The holding's average share of the portfolio's value at the end of each day. */
    average_weight: number;
}
```

<a id="finalytics-conversionrow"></a>

### ConversionRow

One currency conversion.

```typescript
export interface ConversionRow {
    /** The amount sold. */
    sell: number;
    /** Its currency. */
    sell_currency: Currency;
    /** The amount bought. */
    buy: number;
    /** Its currency. */
    buy_currency: Currency;
}
```

<a id="finalytics-correlationstress"></a>

### CorrelationStress

A stress of a covariance matrix: volatilities scaled and correlations moved toward one.

```typescript
export interface CorrelationStress {
    /** The factor every volatility is multiplied by. The default is 1. */
    volatility_scale?: number;
    /**
     * How far every correlation moves toward one, from 0 (not at all, the default) to 1 (all
     * the way).
     */
    correlation_shift?: number;
}
```

<a id="finalytics-covarianceestimate"></a>

### CovarianceEstimate

An estimated covariance matrix, per period of the returns.

```typescript
export interface CovarianceEstimate {
    /** The estimator used. */
    method: CovarianceMethod;
    /**
     * The covariance matrix, labelled like the returns. Scale it to annualise, for example by
     * 252 for daily returns.
     */
    matrix: Matrix;
    /** The shrinkage intensity, between 0 and 1, for the shrinkage estimators. */
    shrinkage?: number | null;
    /** The number of returns per asset used. */
    observations: number;
}
```

<a id="finalytics-covariancemethod"></a>

### CovarianceMethod

How a covariance matrix is estimated from returns.

In JSON it is an object with a `type`, such as `{"type": "ewma", "decay": 0.97}`, or the
type's name alone, such as `"ledoit_wolf"`; an exponentially weighted covariance without a
decay has 0.94.

```typescript
export type CovarianceMethod =
| {
    type: "sample";
```

<a id="finalytics-coverage"></a>

### Coverage

How many securities were measured and how many matched.

```typescript
export interface Coverage {
    /**
     * The securities screened: the universe's, without its benchmark unless
     * `include_benchmark` asks for it.
     */
    securities: number;
    /** Those measured with the history the rules and the sort need. */
    measured: number;
    /** Those measured with too short a history. */
    too_short: number;
    /** Those whose last close is too old: listed, but not measured. */
    stale: number;
    /** Those that meet every rule. */
    passed: number;
    /** The matches kept: those that passed, within the limit. */
    matched: number;
    /**
     * For each measure of the results' `measures`, how many securities that are not stale
     * have no value of it.
     */
    lacking: number[];
    /**
     * For each measure, how many of those without a value have too short a history for it,
     * such as two years of closes for the 3-year return.
     */
    lacking_history?: number[];
}
```

<a id="finalytics-crashwindow"></a>

### CrashWindow

The worst run of twelve months in the history, for the plan's weights.

```typescript
export interface CrashWindow {
    /** Its first month. */
    from: Month;
    /** Its last month. */
    to: Month;
    /** The plan's return over it, in money of the day, such as -0.26. */
    loss: number;
    /** The first month of the plan it is placed at. */
    placed: Month;
}
```

<a id="finalytics-currency"></a>

### Currency

An ISO 4217 currency code, for example "USD".

```typescript
export type Currency = string;
```

<a id="finalytics-currencyattribution"></a>

### CurrencyAttribution

Currency attribution over one or more periods.

```typescript
export interface CurrencyAttribution {
    /** How the periods were linked. */
    linking: Linking;
    /** The portfolio's return in the base currency over all the periods, compounded. */
    portfolio_return: number;
    /** The benchmark's return in the base currency over all the periods, compounded. */
    benchmark_return: number;
    /** `portfolio_return - benchmark_return`, which the linked effects add up to. */
    excess_return: number;
    /** Each segment's market effects, linked. */
    segments: SegmentEffects[];
    /** Each currency's allocation effect, linked, in the order the currencies first appear. */
    currencies: CurrencyEffect[];
    /** The market allocation effects, linked. */
    allocation: number;
    /** The market selection effects, linked. */
    selection: number;
    /** The market interaction effects, linked. */
    interaction: number;
    /** The currency allocation effects, linked. */
    currency: number;
    /** The residuals, linked. */
    residual: number;
    /** Each period's own attribution, before linking. */
    periods: CurrencyPeriodAttribution[];
}
```

<a id="finalytics-currencyattributionperiod"></a>

### CurrencyAttributionPeriod

One period for currency attribution.

```typescript
export interface CurrencyAttributionPeriod {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /** The segments, with returns in their own currencies. */
    segments: CurrencySegment[];
    /** The returns of every currency of the segments, and of the base currency. */
    currencies: CurrencyReturn[];
    /**
     * The portfolio's exposure to each currency after any hedging, adding up to one; without
     * it, each currency's share of the portfolio's segment weights (no hedging).
     */
    portfolio_exposures?: Record<string, number> | null;
    /** The benchmark's exposure to each currency, as for the portfolio. */
    benchmark_exposures?: Record<string, number> | null;
}
```

<a id="finalytics-currencyeffect"></a>

### CurrencyEffect

What one currency added to the excess return.

```typescript
export interface CurrencyEffect {
    /** The currency. */
    currency: Currency;
    /** The portfolio's exposure to it. */
    portfolio_exposure: number;
    /** The benchmark's exposure to it. */
    benchmark_exposure: number;
    /**
     * `(u - v)(f - F)`, with `f` the currency's return including its interest rate and `F` the
     * benchmark's currency return.
     */
    allocation: number;
}
```

<a id="finalytics-currencyperiodattribution"></a>

### CurrencyPeriodAttribution

The currency attribution of one period.

```typescript
export interface CurrencyPeriodAttribution {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /** The portfolio's return in the base currency. */
    portfolio_return: number;
    /** The benchmark's return in the base currency. */
    benchmark_return: number;
    /**
     * The coefficient by which the period's effects are multiplied before they are added up
     * over the periods (one for a single period).
     */
    linking_coefficient: number;
    /** Each segment's market effects, on returns in excess of its currency's cash return. */
    segments: SegmentEffects[];
    /** Each currency's effect. */
    currencies: CurrencyEffect[];
    /** What the model leaves out: the compounding of local and currency returns. */
    residual: number;
}
```

<a id="finalytics-currencyreturn"></a>

### CurrencyReturn

A currency's returns over a period.

```typescript
export interface CurrencyReturn {
    /** The currency. */
    currency: Currency;
    /**
     * The change in its value in the base currency, such as 0.02 when it gains 2% (0 for the
     * base currency).
     */
    spot_return: number;
    /** The return on cash in the currency over the period: its short-term interest rate. */
    cash_return: number;
}
```

<a id="finalytics-currencysegment"></a>

### CurrencySegment

A segment for currency attribution: its returns are in its own currency.

```typescript
export interface CurrencySegment {
    /** The segment's name. */
    name: string;
    /** The currency of the segment's assets. */
    currency: Currency;
    /** The portfolio's weight in the segment at the start of the period. */
    portfolio_weight: number;
    /** The return of the portfolio's holdings in the segment, in the segment's currency. */
    portfolio_return: number;
    /** The benchmark's weight in the segment at the start of the period. */
    benchmark_weight: number;
    /** The return of the benchmark's holdings in the segment, in the segment's currency. */
    benchmark_return: number;
}
```

<a id="finalytics-date"></a>

### Date

A calendar date in ISO 8601 form, for example "2026-11-02".

```typescript
export type Date = string;
```

<a id="finalytics-decimal"></a>

### Decimal

An exact decimal number. Finalytics writes decimals as strings, such as "201.50", and reads strings or numbers.

```typescript
export type Decimal = string;
```

<a id="finalytics-documentfxrates"></a>

### DocumentFxRates

The rates for one currency pair in a portfolio document.

```typescript
export interface DocumentFxRates {
    /** The currency priced. */
    base: Currency;
    /** The currency it is priced in. */
    quote: Currency;
    /** Units of `quote` per unit of `base`. */
    rates: TimeSeries;
}
```

<a id="finalytics-documentmarketdata"></a>

### DocumentMarketData

Market data stored in a portfolio document.

```typescript
export interface DocumentMarketData {
    /** The currency through which FX rates are crossed; USD by default. */
    pivot?: Currency | null;
    /** How many days a price or rate may be carried forward; 5 by default. */
    max_carry_days?: number | null;
    /** Prices by instrument, in each instrument's currency. */
    prices?: Record<string, TimeSeries>;
    /** FX rates: units of `quote` per unit of `base`. */
    fx?: DocumentFxRates[];
}
```

<a id="finalytics-drawdown"></a>

### Drawdown

One fall from a peak, to the trough and, if it happened, back to the peak.

```typescript
export interface Drawdown {
    /** The date of the peak the fall started from. */
    peak: Date;
    /** The date of the lowest point. */
    trough: Date;
    /** The first date at or above the peak again, absent if the series has not recovered. */
    recovery?: Date | null;
    /** The fall from the peak to the trough, as a negative return (-0.25 for a fall of 25%). */
    depth: number;
    /** Calendar days from the peak to the trough. */
    days_to_trough: number;
    /** Calendar days from the trough to the recovery, if it happened. */
    days_to_recovery?: number | null;
}
```

<a id="finalytics-drawdownfigure"></a>

### DrawdownFigure

A conditional drawdown at risk: the mean of the deepest drawdowns of the cumulative returns.

```typescript
export interface DrawdownFigure {
    /** The confidence, such as 0.95. */
    confidence: number;
    /** The mean of the deepest `1 - confidence` of the drawdowns, as a positive fall. */
    value: number;
}
```

<a id="finalytics-effect"></a>

### Effect

A change to the books made by one transaction.

```typescript
export type Effect =
| {
    type: "set_lot";
```

<a id="finalytics-evaluation"></a>

### Evaluation

The figures of given weights on the assets' estimates: those an optimisation gives the
weights it chooses, from the same covariance and expected returns.

```typescript
export interface Evaluation {
    /** The weights by asset, as given. */
    weights: Record<string, number>;
    /** The annual expected return. */
    expected_return: number;
    /** The annual volatility. */
    volatility: number;
    /**
     * `(expected_return - risk_free_rate) / volatility`; absent when the weights are
     * riskless.
     */
    sharpe_ratio?: number | null;
    /** The annual risk-free rate of the Sharpe ratio. */
    risk_free_rate: number;
    /**
     * Each asset's share of the variance, `w_i (Σw)_i / w'Σw`, adding up to one; zero for
     * every asset when the weights are riskless.
     */
    risk_shares: Record<string, number>;
}
```

<a id="finalytics-event"></a>

### Event

One transaction as applied to the books.

```typescript
export interface Event {
    /** The trade date. */
    date: Date;
    /** The transaction. */
    transaction: TransactionId;
    /** Its position in the portfolio's transaction list. */
    index: number;
    /** What it changed. */
    effects: Effect[];
    /** The external flows it caused. */
    flows: Flow[];
    /** How it moved value between holdings. */
    allocations: Allocation[];
}
```

<a id="finalytics-excessreturn"></a>

### ExcessReturn

A return compared with a benchmark's over the same dates.

```typescript
export interface ExcessReturn {
    /** The portfolio's cumulative return. */
    portfolio: number;
    /** The benchmark's cumulative return. */
    benchmark: number;
    /** `portfolio - benchmark`. */
    arithmetic: number;
    /** `(1 + portfolio) / (1 + benchmark) - 1`. */
    geometric: number;
}
```

<a id="finalytics-factorcontribution"></a>

### FactorContribution

One factor's share of a portfolio's volatility.

```typescript
export interface FactorContribution {
    /** The factor's name. */
    factor: string;
    /** The portfolio's exposure to the factor, `(B' w)_k`. */
    exposure: number;
    /**
     * The change in volatility per unit of exposure, `(F x)_k / volatility`. Absent when the
     * portfolio has no volatility.
     */
    marginal?: number | null;
    /**
     * `exposure * marginal`. With the specific component, the components add up to the
     * volatility.
     */
    component: number;
    /** `component / volatility`. Absent when the portfolio has no volatility. */
    share?: number | null;
}
```

<a id="finalytics-factorexposurelimit"></a>

### FactorExposureLimit

Limits on a portfolio's exposure to one factor of the problem's factor model, such as a beta
between 0.9 and 1.1 to the market factor or a neutral exposure to an industry.

```typescript
export interface FactorExposureLimit {
    /** The factor's name, as in the factor model. */
    factor: string;
    /** The smallest exposure allowed. */
    min: number;
    /** The largest exposure allowed. */
    max: number;
    /**
     * Benchmark weights, one per asset. When given, the limits apply to the active exposure,
     * that of `weights - benchmark`.
     */
    benchmark?: number[] | null;
}
```

<a id="finalytics-factormethod"></a>

### FactorMethod

How a factor model was estimated.

```typescript
export type FactorMethod = "regression" | "statistical" | "fundamental";
```

<a id="finalytics-factormodel"></a>

### FactorModel

A linear factor model of asset returns, `r = B f + e`: each asset's exposures `B` to a set of
factors, the covariance `F` of the factor returns, and the variance `d` of each asset's
specific return `e`, which is uncorrelated across assets and with the factors.

The model implies the covariance matrix `B F B' + diag(d)` for the assets. The factor
covariance and the specific variances share units, for example both daily or both
annualised.

```typescript
export interface FactorModel {
    /** The asset labels. */
    assets: string[];
    /**
     * Each asset's exposure to each factor: one row per asset, one entry per factor, in the
     * order of the factor covariance matrix's labels.
     */
    exposures: number[][];
    /** The covariance matrix of the factor returns, labelled with the factor names. */
    factor_covariance: Matrix;
    /** The variance of each asset's specific return, one per asset. */
    specific_variances: number[];
}
```

<a id="finalytics-factormodelestimate"></a>

### FactorModelEstimate

A factor model estimated from returns, with how well it fits each asset.

```typescript
export interface FactorModelEstimate {
    /** How the model was estimated. */
    method: FactorMethod;
    /** The model, in the units of the returns (per period). */
    model: FactorModel;
    /** The fit for each asset, in the model's order. */
    fit: AssetFit[];
    /**
     * The factor returns over the estimation period, one column per factor: the given factor
     * returns for a regression model, the principal component scores for a statistical model,
     * and the estimated factor returns for a fundamental model.
     */
    factor_returns: number[][];
    /** For a statistical model, each component's share of the total variance. */
    explained_variance?: number[] | null;
    /** The number of periods of returns used. */
    observations: number;
}
```

<a id="finalytics-factorrisk"></a>

### FactorRisk

A portfolio's volatility under a factor model, split into each factor's contribution and the
specific (stock-specific) part, and into each asset's contribution (Euler allocation).

```typescript
export interface FactorRisk {
    /** The volatility, `sqrt(x' F x + sum d w^2)`, in the units of the model. */
    volatility: number;
    /** The volatility from the factors alone, `sqrt(x' F x)`. */
    factor_volatility: number;
    /** The volatility from specific returns alone, `sqrt(sum d w^2)`. */
    specific_volatility: number;
    /** Each factor's contribution, in the model's order. */
    factors: FactorContribution[];
    /** The specific part's contribution, `sum d w^2 / volatility`. */
    specific_component: number;
    /** `specific_component / volatility`. Absent when the portfolio has no volatility. */
    specific_share?: number | null;
    /** Each asset's contribution under the model's covariance matrix, in the model's order. */
    assets: RiskContribution[];
}
```

<a id="finalytics-factorshocks"></a>

### FactorShocks

Returns of a factor model's factors, which give each instrument in the model the return its
exposures imply.

```typescript
export interface FactorShocks {
    /** The model. Its assets are instrument identifiers; assets that are not held are ignored. */
    model: FactorModel;
    /** The return of each shocked factor, by factor name. */
    shocks: Record<string, number>;
    /**
     * Whether the factors that are not shocked move too, by their expected return given the
     * shocked ones under the model's factor covariance. Without it, they return zero.
     */
    propagate?: boolean;
}
```

<a id="finalytics-failure"></a>

### Failure

A refit that found no weights.

```typescript
export interface Failure {
    /** The date of the refit. */
    date: Date;
    /** Why there were no weights, such as `infeasible`. */
    reason: string;
}
```

<a id="finalytics-fees"></a>

### Fees

The fee a trade pays, in the instrument's currency: `max(minimum, per_trade + rate * value)`,
rounded to the currency's minor units. Purchases are sized so that the purchase and its fee
fit the target; sales pay the fee from their proceeds.

```typescript
export interface Fees {
    /** A share of the trade's value, such as 0.001 for 10 basis points. */
    rate?: Decimal;
    /** A fixed amount per trade. */
    per_trade?: Decimal;
    /** The smallest fee a trade pays. */
    minimum?: Decimal;
}
```

<a id="finalytics-fieldvalue"></a>

### FieldValue

A value given with a security that the library does not compute: a number, such as a
price-to-earnings ratio, or text, such as a sector.

```typescript
export type FieldValue = number | string;
```

<a id="finalytics-flow"></a>

### Flow

Value entering (positive) or leaving (negative) the portfolio, in `currency`.

```typescript
export interface Flow {
    /** The account. */
    account: AccountId;
    /** The currency of `amount`. */
    currency: Currency;
    /** The value, positive into the portfolio. */
    amount: Decimal;
    /** What kind of flow it was. */
    kind: FlowKind;
}
```

<a id="finalytics-flowkind"></a>

### FlowKind

Why value entered or left the portfolio from outside: the flows that performance measurement
must separate from investment returns.

```typescript
export type FlowKind = "deposit" | "withdrawal" | "transfer_in" | "transfer_out";
```

<a id="finalytics-flowsummary"></a>

### FlowSummary

A flow as simulated: when it falls due and how often.

```typescript
export interface FlowSummary {
    /** The flow's name, such as "Monthly saving". */
    name: string;
    /** Whether money is paid in (a saving) or taken out (spending). */
    saving: boolean;
    /** The amount each time, in today's money unless the flow has a growth rate. */
    amount: number;
    /** How often it falls due; absent for a single payment. */
    every?: Frequency | null;
    /** The first month it falls due. */
    first: Month;
    /** The last month it falls due; absent when it never does. */
    last?: Month | null;
    /** The number of times it falls due. */
    count: number;
    /** Whether it runs until the goal: to the month before a target's date or the first payment. */
    until_goal: boolean;
    /** The growth a year in money of the day, when it does not rise with inflation. */
    growth?: number | null;
    /** For spending, the share of simulations in which every payment of it is made in full. */
    paid_in_full?: number | null;
}
```

<a id="finalytics-flowtiming"></a>

### FlowTiming

When external flows are assumed to happen within the day they are recorded on.

```typescript
export type FlowTiming = "start_of_day" | "end_of_day";
```

<a id="finalytics-formularesult"></a>

### FormulaResult

A formula, with how many securities have no value of it.

```typescript
export interface FormulaResult {
    /** Its name. */
    name: string;
    /** The formula, as given. */
    formula: string;
    /** How many securities that are not stale have no value of it. */
    lacking: number;
}
```

<a id="finalytics-frequency"></a>

### Frequency

How often a regular amount falls due.

```typescript
export type Frequency = "month" | "quarter" | "year";
```

<a id="finalytics-frontierpoint"></a>

### FrontierPoint

A portfolio on the efficient frontier.

```typescript
export interface FrontierPoint {
    /** The expected return. */
    expected_return: number;
    /** The volatility. */
    volatility: number;
    /** The weights, one per asset. */
    weights: number[];
}
```

<a id="finalytics-frontierportfolio"></a>

### FrontierPortfolio

A portfolio on the efficient frontier of `Assets`: its annual expected return and
volatility, and its weights by asset.

```typescript
export interface FrontierPortfolio {
    /** The annual expected return. */
    expected_return: number;
    /** The annual volatility. */
    volatility: number;
    /** The weights, by asset. */
    weights: Record<string, number>;
}
```

<a id="finalytics-givenweights"></a>

### GivenWeights

Given weights valued on the assets' estimates: the report, laid out as an optimisation's,
and the raw figures. Printing it prints the report, and showing it shows the report as a
page.

```typescript
export interface GivenWeights {
    /**
     * The report on the weights: what they are expected to deliver, where their risk comes
     * from, where they sit against each asset (and on the efficient frontier, if asked for)
     * and how they would have done on the assets' history.
     */
    report: OptimisationReport;
    /** The raw figures. */
    results: Evaluation;
}
```

<a id="finalytics-goal"></a>

### Goal

The goal a plan is tested against.

```typescript
export type Goal =
| {
    type: "target";
```

<a id="finalytics-grid"></a>

### Grid

A table of numbers with labelled rows and columns, for a heatmap.

```typescript
export interface Grid {
    /** What the grid is, such as "Monthly returns". */
    name: string;
    /** What its values measure. */
    unit: Unit;
    /** The row labels. */
    rows: string[];
    /** The column labels. */
    columns: string[];
    /** One row of values per row label, one value per column; absent where there is none. */
    values: ((number | null)[])[];
}
```

<a id="finalytics-groupcontribution"></a>

### GroupContribution

What a group of holdings (an asset class, a region) added to the portfolio's return.

```typescript
export interface GroupContribution {
    /** The group. */
    group: string;
    /** The sum of its holdings' contributions. */
    contribution: number;
    /** The sum of its holdings' average weights. */
    average_weight: number;
}
```

<a id="finalytics-groupimpact"></a>

### GroupImpact

What a scenario does to one group of holdings.

```typescript
export interface GroupImpact {
    /** The group. */
    group: string;
    /** Its value before, in the base currency. */
    value_before: number;
    /** Its value after. */
    value_after: number;
    /** `value_after - value_before`. */
    change: number;
    /** Its share of the portfolio's value before. */
    weight_before: number;
    /**
     * Its share of the portfolio's value after, absent if the portfolio is worth nothing
     * after.
     */
    weight_after?: number | null;
    /**
     * `change` as a share of the portfolio's value before: the groups' contributions add up to
     * the portfolio's return.
     */
    contribution: number;
}
```

<a id="finalytics-grouplimit"></a>

### GroupLimit

Limits on the total weight of a group of assets, such as an asset class or a region.

```typescript
export interface GroupLimit {
    /** The group's name, used in messages. */
    name: string;
    /** The labels of the assets in the group. */
    assets: string[];
    /** The smallest total weight allowed. */
    min: number;
    /** The largest total weight allowed. */
    max: number;
}
```

<a id="finalytics-grouprisk"></a>

### GroupRisk

A group's share of a portfolio's volatility.

```typescript
export interface GroupRisk {
    /** The group. */
    group: string;
    /** The sum of its assets' weights. */
    weight: number;
    /** The sum of its assets' components. */
    component: number;
    /** The sum of its assets' shares, absent when the portfolio has no risk. */
    share?: number | null;
}
```

<a id="finalytics-grouprow"></a>

### GroupRow

A group of assets with limits on its total weight.

```typescript
export interface GroupRow {
    /** The group's name, such as "Energy". */
    name: string;
    /** Its assets. */
    assets: string[];
    /** The total weight of its assets. */
    weight: number;
    /** The smallest total weight allowed. */
    min: number;
    /** The largest total weight allowed. */
    max: number;
    /** The sum of its assets' shares of the risk. */
    risk_share: number;
}
```

<a id="finalytics-groupshock"></a>

### GroupShock

A return for every instrument with a classification, such as all equities.

```typescript
export interface GroupShock {
    /** The classification scheme, such as `asset_class`. */
    scheme: string;
    /** The classification, such as `equity`. */
    value: string;
    /** The return of each instrument in the group, in its own currency: -0.2 for a fall of 20%. */
    shock: number;
}
```

<a id="finalytics-headline"></a>

### Headline

The headline figures of the overview.

```typescript
export interface Headline {
    /** The portfolio's value in the base currency. */
    value: number;
    /** The gain over the last valuation period: the change in value less the flows. */
    period_gain?: number | null;
    /** The time-weighted return over the last valuation period. */
    period_return?: number | null;
    /** The time-weighted return year to date. */
    year_to_date?: number | null;
    /** The time-weighted return since the first valuation. */
    since_inception?: number | null;
    /** The annualised return since the first valuation, for a year or more. */
    since_inception_annualised?: number | null;
    /** The annualised volatility. */
    volatility: number;
    /** The Sharpe ratio. */
    sharpe_ratio?: number | null;
    /** The maximum drawdown, a negative return. */
    max_drawdown: number;
    /** The confidence of the value at risk and expected shortfall: the first of the options. */
    confidence?: number | null;
    /** The historical value at risk over one valuation period. */
    value_at_risk?: Amount | null;
    /** The historical expected shortfall over one valuation period. */
    expected_shortfall?: Amount | null;
}
```

<a id="finalytics-histogram"></a>

### Histogram

The distribution of a set of observations.

```typescript
export interface Histogram {
    /** What the observations are. */
    name: string;
    /** What they measure. */
    unit: Unit;
    /** Bins of equal width from the smallest observation to the largest. */
    bins: Bin[];
}
```

<a id="finalytics-historicalwindow"></a>

### HistoricalWindow

A historical period to replay on today's holdings.

```typescript
export interface HistoricalWindow {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /**
     * Instruments whose prices stand in for others' over the period, such as an index for a
     * fund that did not exist then: the proxy of each instrument, by instrument.
     */
    proxies?: Record<string, InstrumentId>;
}
```

<a id="finalytics-history"></a>

### History

A holdout's two periods together, for the charts of the whole history.

```typescript
export interface History {
    /** The first period out of sample. */
    split: Date;
    /** The allocation over the whole history. */
    strategy: RunReport;
    /** The benchmark over the whole history. */
    benchmark?: RunReport | null;
}
```

<a id="finalytics-holdingimpact"></a>

### HoldingImpact

What a scenario does to one holding: an instrument, or cash in one currency, added up across
accounts.

```typescript
export interface HoldingImpact {
    /** The holding. */
    holding: HoldingKey;
    /** Its value before, in the base currency. */
    value_before: number;
    /** Its share of the portfolio's value before. */
    weight_before: number;
    /** Its return in its own currency. */
    return_local: number;
    /** The change in its currency's value in the base currency. */
    currency_return: number;
    /** Its return in the base currency, `(1 + return_local)(1 + currency_return) - 1`. */
    return_base: number;
    /** Its value after, in the base currency. */
    value_after: number;
    /**
     * Its share of the portfolio's value after, absent if the portfolio is worth nothing
     * after.
     */
    weight_after?: number | null;
    /** `value_after - value_before`. */
    change: number;
    /** How its return was set. */
    source: ShockSource;
}
```

<a id="finalytics-holdingkey"></a>

### HoldingKey

A holding: an instrument's identifier, such as "SPY", or a currency's code followed by " cash", such as "USD cash".

```typescript
export type HoldingKey = string;
```

<a id="finalytics-holdingline"></a>

### HoldingLine

One holding's values, trades and income on each date of a `HoldingSeries`.

Trades, income and expenses are those since the previous date. Value moved between holdings
(a purchase moves cash into a share) appears as a trade out of one line and into another, so
trades across all lines add up to the portfolio's external flows plus its income. Fees and
taxes charged to an account rather than to a holding are expenses of its cash: they lower the
cash's value without being a loss on it, so a holding's gain on a date is its change in value
less its trades, plus its income and its expenses.

```typescript
export interface HoldingLine {
    /** The holding. */
    holding: HoldingKey;
    /** The holding's own currency: the instrument's, or the cash currency. */
    currency: Currency;
    /** The value at the end of each date, in `currency`. */
    values_local: Decimal[];
    /** The value at the end of each date, in the base currency. */
    values_base: Decimal[];
    /** The value moved in since the previous date, in `currency`. */
    trades_local: Decimal[];
    /** The value moved in since the previous date, in the base currency. */
    trades_base: Decimal[];
    /** The income earned since the previous date, in `currency`. */
    income_local: Decimal[];
    /** The income earned since the previous date, in the base currency. */
    income_base: Decimal[];
    /** The fees and taxes paid out of the holding since the previous date, in `currency`. */
    expenses_local: Decimal[];
    /**
     * The fees and taxes paid out of the holding since the previous date, in the base
     * currency.
     */
    expenses_base: Decimal[];
}
```

<a id="finalytics-holdingmove"></a>

### HoldingMove

A holding's weight before the trades, at the target and after.

```typescript
export interface HoldingMove {
    /** The holding: an instrument's identifier, or a currency followed by ` cash`. */
    holding: string;
    /** The weight before the trades. */
    before: number;
    /** The target weight; base-currency cash takes what the targets leave. */
    target: number;
    /** The weight after the trades. */
    after: number;
    /** The value after the trades. */
    value_after_base: number;
}
```

<a id="finalytics-holdingreturns"></a>

### HoldingReturns

Returns of holdings in the base currency between consecutive dates.

```typescript
export interface HoldingReturns {
    /** The holdings, in the order of the table's columns. */
    holdings: HoldingKey[];
    /** The date each return ends on: every date given but the first. */
    dates: Date[];
    /**
     * The returns, one column per holding, labelled with the instrument identifier or, for
     * cash, the currency code followed by " cash".
     */
    table: ReturnsTable;
    /** Prices and rates carried forward from earlier dates. */
    warnings: ValidationReport;
}
```

<a id="finalytics-holdingriskreturn"></a>

### HoldingRiskReturn

One holding's weight, expected return and risk.

```typescript
export interface HoldingRiskReturn {
    /** The holding. */
    holding: HoldingKey;
    /** Its share of the portfolio's value. */
    weight: number;
    /** Its annual expected return: given in the options, or its historical mean. */
    expected_return: number;
    /** Whether the expected return was given rather than the historical mean. */
    expected_return_given: boolean;
    /** Its annualised volatility. */
    volatility: number;
    /** Its share of the portfolio's volatility. */
    risk_share?: number | null;
}
```

<a id="finalytics-holdingrow"></a>

### HoldingRow

One holding, added up across accounts.

```typescript
export interface HoldingRow {
    /** The holding. */
    holding: HoldingKey;
    /** The instrument's name, if it has one. */
    name?: string | null;
    /** The instrument's kind; absent for cash. */
    kind?: InstrumentKind | null;
    /** The holding's currency. */
    currency: Currency;
    /** The quantity held, or the cash balance. */
    quantity: Decimal;
    /** The price per unit, in the instrument's currency; absent for cash. */
    price?: Decimal | null;
    /** The value in the holding's currency. */
    value_local: Decimal;
    /** The value in the base currency. */
    value_base: Decimal;
    /** The cost basis in the instrument's currency; absent for cash. */
    cost_local?: Decimal | null;
    /** The cost basis in the base currency; absent for cash. */
    cost_base?: Decimal | null;
    /** `value_local - cost_local`. */
    unrealised_local?: Decimal | null;
    /** `value_base - cost_base`, which includes the currency effect. */
    unrealised_base?: Decimal | null;
    /** `unrealised_base / cost_base`, when the cost is above zero. */
    unrealised_return?: number | null;
    /** The share of the portfolio's value. */
    weight: number;
    /** The instrument's classifications. */
    classifications: Record<string, string>;
}
```

<a id="finalytics-holdingseries"></a>

### HoldingSeries

Values, value moved in and income per holding over dates, in the holding's currency and in
the base currency: the input to contribution analysis.

```typescript
export interface HoldingSeries {
    /** The dates. */
    dates: Date[];
    /** The base currency. */
    base_currency: Currency;
    /** One line per instrument and per cash currency held at any time, in key order. */
    lines: HoldingLine[];
    /** Prices and rates carried forward from earlier dates. */
    warnings: ValidationReport;
}
```

<a id="finalytics-holdingsperiod"></a>

### HoldingsPeriod

Gains realised and income received within the report's period, from its start to its date.

```typescript
export interface HoldingsPeriod {
    /** The period's first day: disposals and payments on it count. */
    start: Date;
    /** The period's last day, the report's date. */
    end: Date;
    /** Gains realised within the period. */
    realised: RealisedSummary;
    /** Income received within the period, net of tax withheld. */
    income: IncomeSummary;
}
```

<a id="finalytics-holdingsreport"></a>

### HoldingsReport

The holdings report.

```typescript
export interface HoldingsReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The portfolio's value in the base currency. */
    value: Decimal;
    /** Each holding, added up across accounts, largest first. */
    holdings: HoldingRow[];
    /** Each account's holdings, in the portfolio's order. */
    accounts: AccountHoldings[];
    /** The open tax lots, by account and instrument. */
    lots: LotRow[];
    /**
     * The value split by holding, currency, instrument kind, account and each scheme of the
     * options.
     */
    breakdowns: Breakdown[];
    /** How concentrated the instruments are. */
    concentration: Concentration;
    /** Gains realised to date. */
    realised: RealisedSummary;
    /** Income received to date. */
    income: IncomeSummary;
    /**
     * Gains realised and income received within the report's period, as the performance
     * report's flows count them: the same as to date unless the report starts after the first
     * transaction.
     */
    period?: HoldingsPeriod | null;
}
```

<a id="finalytics-holdingsrisk"></a>

### HoldingsRisk

The current holdings' risk, from the covariance of their returns.

```typescript
export interface HoldingsRisk {
    /**
     * The first date of the returns: the first valuation date on which every current holding
     * has a price.
     */
    start: Date;
    /** The number of returns. */
    observations: number;
    /** How the covariance was estimated. */
    method: CovarianceMethod;
    /**
     * The annualised covariance of the holdings' returns in the base currency, labelled by
     * holding.
     */
    covariance: Matrix;
    /** Their correlations, for a heatmap. */
    correlation: Grid;
    /** The annualised volatility of the current weights, by holding. */
    decomposition: RiskDecomposition;
    /** The decomposition added up by each scheme of the options. */
    groups: SchemeRisk[];
    /** The ex-ante tracking error against the benchmark weights of the options, by holding. */
    tracking_error?: RiskDecomposition | null;
}
```

<a id="finalytics-household"></a>

### Household

A household's performance over a period: consolidated, by account and by group of accounts.

```typescript
export interface Household {
    /** The start of the period: the first date. */
    start: Date;
    /** The end of the period: the last date. */
    end: Date;
    /** Every account together. */
    household: AccountsPerformance;
    /** Each account, in the portfolio's order. */
    accounts: AccountsPerformance[];
    /** Each scheme the options group by. */
    groups: SchemePerformance[];
    /** Prices and rates carried forward, and returns that could not be measured. */
    warnings: ValidationReport;
}
```

<a id="finalytics-householdoptions"></a>

### HouseholdOptions

Options for `household`.

```typescript
export interface HouseholdOptions {
    /**
     * Classification schemes of the accounts to group them by, such as `owner` or `wrapper`.
     * Accounts without the classification are in the group "unclassified".
     */
    group_by?: string[];
    /** When external flows happen within the day: at the start (the default) or the end. */
    timing?: FlowTiming;
}
```

<a id="finalytics-incomesummary"></a>

### IncomeSummary

Income received up to the report's date, net of tax withheld, in the base currency.

```typescript
export interface IncomeSummary {
    /** All income. */
    total: Decimal;
    /** By calendar year. */
    by_year: Record<string, Decimal>;
    /** By holding: the instrument that paid it, or cash for interest on cash. */
    by_holding: Record<string, Decimal>;
}
```

<a id="finalytics-inflation"></a>

### Inflation

How prices rise, for amounts in today's money.

```typescript
export type Inflation =
| {
    type: "rate";
```

<a id="finalytics-inflationbasis"></a>

### InflationBasis

How the plan's amounts are kept in today's money.

```typescript
export type InflationBasis =
| {
    type: "rate";
```

<a id="finalytics-instrument"></a>

### Instrument

Something a portfolio can hold, other than cash.

```typescript
export interface Instrument {
    /** The caller's identifier. */
    id: InstrumentId;
    /** A display name. */
    name?: string | null;
    /** The kind of instrument. */
    kind: InstrumentKind;
    /** The currency it is priced and traded in. */
    currency: Currency;
    /**
     * The value of one unit at a price of one: 1 for shares, the contract size for futures
     * and options.
     */
    multiplier?: Decimal;
    /** Classifications by scheme, for example `asset_class: Equity`, `region: Europe`. */
    classifications?: Record<string, string>;
    /** Other identifiers by scheme, for example `isin` or `ticker`. */
    identifiers?: Record<string, string>;
}
```

<a id="finalytics-instrumentid"></a>

### InstrumentId

Identifies an instrument, for example `"AAPL"` or `"US0378331005"`.

Identifiers are non-empty, have no leading or trailing whitespace, and are compared
exactly (case matters).

```typescript
export type InstrumentId = string;
```

<a id="finalytics-instrumentkind"></a>

### InstrumentKind

What kind of instrument something is.

Every kind is held and valued the same way, as a quantity times a price times a multiplier.
Derivatives and bonds are supplied already priced; Finalytics does not price them.

```typescript
export type InstrumentKind = "equity" | "fund" | "bond" | "future" | "option" | "fx_forward" | "other";
```

<a id="finalytics-issue"></a>

### Issue

One problem found while validating input.

```typescript
export interface Issue {
    /** Where the problem is, for example `transactions[14].quantity`. */
    path: string;
    /** What is wrong, in plain words. */
    message: string;
    /** How serious the problem is. */
    severity: Severity;
}
```

<a id="finalytics-keepcash"></a>

### KeepCash

The cash to keep as it is: every currency's (`true`), or that of the currencies named, such
as `["USD"]`. Rebalancing keeps it as it is, the targets sharing the rest of the value;
optimisation holds it at its current weight and optimises the rest.

```typescript
export type KeepCash = boolean | string[];
```

<a id="finalytics-keptcash"></a>

### KeptCash

Cash a rebalancing kept as it was.

```typescript
export interface KeptCash {
    /** The currency. */
    currency: Currency;
    /** The amount kept, in the accounts rebalanced together. */
    amount: Decimal;
    /** Its value in the base currency. */
    value_base: Decimal;
    /** Its share of the accounts' value, which the targets leave to it. */
    weight: number;
}
```

<a id="finalytics-linkage"></a>

### Linkage

How clusters are joined in hierarchical risk parity.

```typescript
export type Linkage = "single" | "complete" | "average" | "ward";
```

<a id="finalytics-linking"></a>

### Linking

How the effects of several periods are linked so that they add up to the excess return over
all of them.

```typescript
export type Linking = "carino" | "menchero";
```

<a id="finalytics-lot"></a>

### Lot

Units of one instrument acquired together, with their remaining cost basis.

Costs are totals for the remaining quantity, in the instrument's currency (`cost_local`) and in
the portfolio's base currency at the acquisition rate (`cost_base`). Keeping totals rather than
unit costs means that splitting a lot never loses or creates cost.

```typescript
export interface Lot {
    /** The lot's identifier. */
    id: LotId;
    /** The account holding it. */
    account: AccountId;
    /** The instrument. */
    instrument: InstrumentId;
    /** The remaining quantity, greater than zero while the lot is open. */
    quantity: Decimal;
    /** The remaining cost basis in the instrument's currency, including acquisition fees. */
    cost_local: Decimal;
    /** The remaining cost basis in the base currency. */
    cost_base: Decimal;
    /** The acquisition date, which sets the holding period. */
    acquired: Date;
    /**
     * The transaction that opened the lot (or the earliest one, for lots carried through
     * corporate actions).
     */
    opened_by: TransactionId;
    /** The order in which lots were opened, used to break ties. */
    sequence: number;
}
```

<a id="finalytics-lotid"></a>

### LotId

Identifies a tax lot: the transaction that opened it, followed by `/` and the corporate
action that derived it, if any.

Identifiers are non-empty, have no leading or trailing whitespace, and are compared
exactly (case matters).

```typescript
export type LotId = string;
```

<a id="finalytics-lotmethod"></a>

### LotMethod

How sales choose which tax lots to close.

```typescript
export type LotMethod = "fifo" | "lifo" | "highest_cost" | "lowest_cost" | "average_cost" | "specific_id";
```

<a id="finalytics-lotpick"></a>

### LotPick

A lot named by a sale or transfer, for specific identification.

```typescript
export interface LotPick {
    /** The lot. */
    lot: LotId;
    /** The number of units to take from it. */
    quantity: Decimal;
}
```

<a id="finalytics-lotrow"></a>

### LotRow

One open tax lot, valued.

```typescript
export interface LotRow {
    /** The account. */
    account: AccountId;
    /** The instrument. */
    instrument: InstrumentId;
    /** The lot. */
    lot: LotId;
    /** The acquisition date. */
    acquired: Date;
    /** The quantity. */
    quantity: Decimal;
    /** The cost basis in the base currency. */
    cost_base: Decimal;
    /** The value in the base currency. */
    value_base: Decimal;
    /** `value_base - cost_base`. */
    unrealised_base: Decimal;
    /** `unrealised_base / cost_base`, when the cost is above zero. */
    unrealised_return?: number | null;
    /** Calendar days held to the report's date. */
    holding_days: number;
    /** Whether it has been held for longer than the long-term period. */
    long_term: boolean;
}
```

<a id="finalytics-lotsale"></a>

### LotSale

The part of a lot that a sale would close.

```typescript
export interface LotSale {
    /** The lot. */
    lot: LotId;
    /** When the lot was acquired. */
    acquired: Date;
    /** The units the sale would take from it. */
    quantity: Decimal;
    /** The cost basis of those units in the instrument's currency. */
    cost_local: Decimal;
    /** The cost basis of those units in the base currency. */
    cost_base: Decimal;
}
```

<a id="finalytics-matrix"></a>

### Matrix

A square matrix with labelled rows and columns, stored row by row; a covariance or
correlation matrix.

```typescript
export interface Matrix {
    labels: string[];
    values: number[];
}
```

<a id="finalytics-meanmethod"></a>

### MeanMethod

How the assets' mean returns are estimated from their returns.

In JSON it is an object with a `type`, such as `{"type": "ewma", "decay": 0.97}`, or the
type's name alone, such as `"james_stein"`; an exponentially weighted mean without a decay
has 0.94.

```typescript
export type MeanMethod =
| {
    type: "historical";
```

<a id="finalytics-measure"></a>

### Measure

A measure a screen computes for each security, from its own prices up to the date measured
to. Each is named in options and results by its snake-case name, such as `"return_1y"`.

```typescript
export type Measure = "return_1w" | "return_1m" | "return_3m" | "return_6m" | "return_ytd" | "return_1y" | "return_3y" | "return_5y" | "volatility" | "volatility_3y" | "downside_deviation" | "max_drawdown" | "max_drawdown_3y" | "drawdown" | "sharpe_ratio" | "sharpe_ratio_3y" | "sortino_ratio" | "calmar_ratio" | "value_at_risk" | "beta" | "correlation" | "relative_return_6m" | "relative_return_1y" | "price_to_sma_20" | "price_to_sma_50" | "price_to_sma_200" | "sma_50_above_200" | "rsi_14" | "macd_histogram" | "from_52w_high" | "from_52w_low" | "volatility_20d" | "dividend_yield" | "average_volume_20d" | "volume_ratio" | "dollar_volume_20d" | "observations" | "last_price" | "stale";
```

<a id="finalytics-measurespread"></a>

### MeasureSpread

The spread of one measure, or of a numeric field a rule tests, across the securities that
are not stale: the smallest and largest values and the quartiles, with the values the rules
compare it with.

```typescript
export interface MeasureSpread {
    /** The measure's or the field's name. */
    name: string;
    /** Its name in words, such as "1-year return". */
    label: string;
    /** Its group, such as "Returns", or "Fields". */
    group: string;
    /** How a measure is shown; absent for a field. */
    unit?: MeasureUnit | null;
    /** How many securities have a value. */
    count: number;
    /** How many that are not stale have none. */
    lacking: number;
    /** How many of those without a value have too short a history for it; none for a field. */
    lacking_history?: number;
    /** For a measure that is yes or no, how many securities have yes. */
    yes?: number | null;
    /** The smallest value. */
    min?: number | null;
    /** The lower quartile, interpolated linearly between values. */
    lower_quartile?: number | null;
    /** The median. */
    median?: number | null;
    /** The upper quartile. */
    upper_quartile?: number | null;
    /** The largest value. */
    max?: number | null;
    /** The values the rules compare it with. */
    thresholds: number[];
}
```

<a id="finalytics-measureunit"></a>

### MeasureUnit

How a measure is shown: as a percentage, a ratio, a price, an index from 0 to 100, a count or
a yes or no.

```typescript
export type MeasureUnit = "fraction" | "ratio" | "price" | "index" | "count" | "flag";
```

<a id="finalytics-method"></a>

### Method

An optimisation method, by name. The parameters some methods need are given beside it, in
`OptimiseOptions` or with the builder's methods.

```typescript
export type Method = "minimum_variance" | "mean_variance" | "robust_mean_variance" | "target_return" | "target_volatility" | "maximum_sharpe" | "maximum_diversification" | "minimum_cvar" | "minimum_semivariance" | "minimum_cdar" | "equal_weight" | "inverse_volatility" | "risk_parity" | "risk_budgeting" | "hierarchical_risk_parity";
```

<a id="finalytics-methodinfo"></a>

### MethodInfo

An optimisation method and what it takes.

```typescript
export interface MethodInfo {
    /** The method. */
    method: Method;
    /** Its name in words, such as "Maximum Sharpe". */
    label: string;
    /** What it does, in a sentence. */
    description: string;
    /** Whether it takes limits on weights; those that do not are fully invested and long only. */
    takes_limits: boolean;
    /** Whether its weights can be resampled. */
    resamples: boolean;
    /** Whether it is measured on the periods' returns as scenarios, rather than on estimates. */
    uses_scenarios: boolean;
    /**
     * Its own parameters, such as a target or a confidence, then the options every method
     * takes.
     */
    parameters: MethodParameter[];
    /** The limits on weights it takes; none for the methods that take no limits. */
    limits: MethodParameter[];
}
```

<a id="finalytics-methodparameter"></a>

### MethodParameter

One option an optimisation method takes, with the values it accepts.

```typescript
export interface MethodParameter {
    /** The option's name, as in `OptimiseOptions` and the builders, such as `confidence`. */
    name: string;
    /** The kind of value it takes. */
    kind: ParameterKind;
    /** Whether the method needs it. */
    required: boolean;
    /** The value used when it is not given, if a number. */
    default?: number | null;
    /** The value used when it is not given, if a choice. */
    default_choice?: string | null;
    /** The smallest value allowed, for numbers and the numbers by asset. */
    min?: number | null;
    /** Whether `min` itself is not allowed. */
    min_exclusive: boolean;
    /** The largest value allowed. */
    max?: number | null;
    /** Whether `max` itself is not allowed. */
    max_exclusive: boolean;
    /** The names allowed, for a choice. */
    choices: string[];
    /** The other options it needs, such as `current_weights` for a turnover limit. */
    needs: string[];
    /** What it does, in a sentence. */
    description: string;
}
```

<a id="finalytics-mix"></a>

### Mix

What the money is invested in: fixed weights, or weights that change along a glide path.
The weights are restored every month.

```typescript
export type Mix =
| {
    type: "fixed";
```

<a id="finalytics-mixoutcome"></a>

### MixOutcome

One mix compared with the plan's on the same simulations.

```typescript
export interface MixOutcome {
    /** The mix's name. */
    name: string;
    /** The weights by asset at the start. */
    weights: [string, number][];
    /** Whether the weights change along a glide path. */
    glide_path: boolean;
    /** The outcome. */
    outcome: Outcome;
    /** The value at the end across the simulations. */
    end: Percentiles;
    /** For payments, the month by which 1 simulation in 10 has run short. */
    runs_out_weak?: Month | null;
    /** The risk of the mix. */
    risk: MixRisk;
    /** The mix when markets do worse. */
    what_if: WhatIf;
}
```

<a id="finalytics-mixrisk"></a>

### MixRisk

The risk of a mix over the plan, before any money is paid in or out.

```typescript
export interface MixRisk {
    /** The typical return a year after inflation and fees: the median across the simulations. */
    typical_return: number;
    /**
     * The 5th percentile of twelve-month returns: 1 year in 20 is worse; absent for a plan
     * shorter than a year.
     */
    worst_year?: number | null;
    /** The median of each simulation's largest fall from a peak. */
    largest_fall: number;
    /** The volatility a year of the mix's returns in the history, in money of the day. */
    volatility: number;
}
```

<a id="finalytics-model"></a>

### Model

How the simulations draw each month's returns.

```typescript
export type Model =
| {
    type: "bootstrap";
```

<a id="finalytics-money"></a>

### Money

An exact decimal amount in one currency.

Arithmetic between amounts checks that the currencies match and that no overflow occurs.
Amounts keep their full precision until they are rounded.

```typescript
export interface Money {
    amount: Decimal;
    currency: Currency;
}
```

<a id="finalytics-moneyfigures"></a>

### MoneyFigures

A run followed with money: a value at the start, and money paid in and taken out.

```typescript
export interface MoneyFigures {
    /** The value on the date the run enters. */
    start_value: number;
    /** The money paid in during the run, after the start. */
    paid_in: number;
    /** The money taken out during the run; a withdrawal takes no more than the value. */
    taken_out: number;
    /** The value at the end of the last period. */
    ending_value: number;
    /**
     * The money-weighted return: the internal rate of return of the start value, the flows and
     * the ending value, a year (actual/365); absent when there is none.
     */
    money_weighted_return?: number | null;
    /** The number of flows that fell in the run. */
    flows: number;
}
```

<a id="finalytics-month"></a>

### Month

A calendar month, written as "2030-06"; a date such as "2030-06-15" reads as its month.

```typescript
export type Month = string;
```

<a id="finalytics-monthchange"></a>

### MonthChange

A change of a month that reaches, or keeps, the chance wanted.

```typescript
export interface MonthChange {
    /** What is changed: the target's date, or the month of the first payment. */
    what: string;
    /** The month in the plan. */
    from: Month;
    /** The month that gives the chance wanted, saving until then. */
    to: Month;
}
```

<a id="finalytics-monthflows"></a>

### MonthFlows

External flows and income in one calendar month, in the base currency.

```typescript
export interface MonthFlows {
    /** The month, as `YYYY-MM`. */
    month: string;
    /** Deposits and transfers in, positive. */
    inflows: number;
    /** Withdrawals and transfers out, negative. */
    outflows: number;
    /** Income, net of tax withheld. */
    income: number;
}
```

<a id="finalytics-namedscenario"></a>

### NamedScenario

A scenario with a name, such as "Equities -20%" or "2008".

```typescript
export interface NamedScenario {
    /** The name. */
    name: string;
    /** What it does. */
    scenario: ScenarioDefinition;
    /** A stress of the covariance for the risk after this scenario, as in a crisis. */
    stress?: CorrelationStress | null;
}
```

<a id="finalytics-nearmiss"></a>

### NearMiss

A security that fails just one rule.

```typescript
export interface NearMiss {
    /** The security. */
    id: string;
    /** The place in the report's `rules` of the rule it fails. */
    rule: number;
    /** Its value in each column, in the order of the report's `columns`. */
    values: (FieldValue | null)[];
}
```

<a id="finalytics-objective"></a>

### Objective

What an optimisation seeks.

```typescript
export type Objective =
| {
    type: "minimum_variance";
```

<a id="finalytics-op"></a>

### Op

How a rule compares a security's value: with a number, a range or a list of text values.

```typescript
export type Op = "<" | "<=" | ">" | ">=" | "between" | "in" | "not_in";
```

<a id="finalytics-optimisation"></a>

### Optimisation

An optimisation: the optimisation report on the weights chosen, and the raw results.
Printing it prints the report, and showing it shows the report as a page.

```typescript
export interface Optimisation {
    /**
     * The optimisation report: the weights, what they are expected to deliver, where their
     * risk comes from, where they sit against each asset (and on the efficient frontier, if
     * asked for) and the history they were fitted to.
     */
    report: OptimisationReport;
    /**
     * The raw results: the weights by asset, their expected return, volatility and Sharpe
     * ratio, and the solver's solution.
     */
    results: OptimisationResult;
}
```

<a id="finalytics-optimisationcontext"></a>

### OptimisationContext

What an optimisation report describes: the method, the data and estimates, the limits and
how the solver ended.

```typescript
export interface OptimisationContext {
    /** The engine's version. */
    version: string;
    /**
     * A SHA-256 digest of every input, in hexadecimal: the estimates, the method with its
     * parameters and limits, and any weights given. The same inputs give the same report, and
     * different inputs a different digest; the title, which printing can also give, is not
     * one of them.
     */
    digest: string;
    /** The method's name, such as "Maximum Sharpe". */
    method: string;
    /** What the method does, in a sentence. */
    description: string;
    /** How the optimisation ended, such as `optimal`. */
    status: string;
    /** How it was solved, such as `interior_point`. */
    solver: string;
    /** The solver's iterations. */
    iterations: number;
    /** The assets optimised over, other than the risk-free asset. */
    assets: string[];
    /** The risk-free asset, if one of the assets is. */
    risk_free_asset?: string | null;
    /** The annual risk-free rate of the Sharpe ratio. */
    risk_free_rate: number;
    /** The number of return periods in a year. */
    periods_per_year: number;
    /** The covariance estimator. */
    covariance: CovarianceMethod;
    /** The estimator of the expected returns: `None` when some were given. */
    mean_returns?: MeanMethod | null;
    /** The limits on the weights, in words. */
    limits: string[];
    /** The date the first return period ends on. */
    start: Date;
    /** The date the last return period ends on. */
    end: Date;
    /** For resampled optimisation, how the weights were averaged. */
    resampling?: ResamplingSummary | null;
    /**
     * Whether the weights were given rather than optimised: the report then values them on
     * the estimates, and the limits are those of the frontier.
     */
    given?: boolean;
}
```

<a id="finalytics-optimisationhistory"></a>

### OptimisationHistory

How the weights, and equal weight, would have done on the history they were fitted to.

```typescript
export interface OptimisationHistory {
    /** The weights, held from the start and rebalanced quarterly. */
    weights: RunReport;
    /** Equal weight, held the same way. */
    equal_weight: RunReport;
}
```

<a id="finalytics-optimisationreport"></a>

### OptimisationReport

The weights an optimisation chose, with what they are expected to deliver, where their risk
comes from, where they sit against each asset and on the efficient frontier, and how they
would have done on the history they were fitted to.

```typescript
export interface OptimisationReport {
    /** What was optimised and how. */
    context: OptimisationContext;
    /** What the weights are expected to deliver; absent when the optimisation found no weights. */
    summary?: OptimisationSummary | null;
    /**
     * Each asset's weight, expected return, volatility and share of the risk, the largest
     * weight first.
     */
    allocation: AllocationRow[];
    /**
     * Each group of assets with limits on its total weight, with that weight and its share of
     * the risk, in name order.
     */
    groups?: GroupRow[];
    /** The correlation of the assets' returns, from the covariance the optimisation used. */
    correlation: Grid;
    /**
     * The efficient frontier under the same limits, from the least volatile portfolio to the
     * one with the largest expected return; empty unless the optimisation asked for it.
     */
    frontier: RiskReturn[];
    /** The equal-weight portfolio of the same assets, for comparison. */
    equal_weight?: RiskReturn | null;
    /**
     * How the weights, and equal weight, would have done on the history they were fitted to:
     * held from the start and rebalanced quarterly, without trading costs.
     */
    history?: OptimisationHistory | null;
    /** The trades from the weights held now to these, when the current weights were given. */
    trades?: TradesSummary | null;
    /**
     * The report's title, such as "Six asset classes": shown when it is printed or laid out as
     * a page.
     */
    name?: string | null;
}
```

<a id="finalytics-optimisationresult"></a>

### OptimisationResult

The result of an optimisation over assets: the weights by asset, with their expected return,
volatility and Sharpe ratio, and the solver's full solution.

```typescript
export interface OptimisationResult {
    /** How the optimisation ended. */
    status: SolveStatus;
    /** The weights by asset, absent when there is no solution. */
    weights?: Record<string, number> | null;
    /** The annual expected return of the weights. */
    expected_return?: number | null;
    /** The annual volatility of the weights. */
    volatility?: number | null;
    /** `(expected_return - risk_free_rate) / volatility`. */
    sharpe_ratio?: number | null;
    /** The annual risk-free rate used. */
    risk_free_rate: number;
    /**
     * The solver's solution, with the objective, its value and the work done. With
     * resampling, it is the solution for the estimates themselves, and `weights` are the
     * average in `resampled`.
     */
    solution: Solution;
    /**
     * For resampled optimisation, the average weights, how much each varied across the
     * simulated histories, and how many were solved.
     */
    resampled?: ResampledSolution | null;
}
```

<a id="finalytics-optimisationsummary"></a>

### OptimisationSummary

What the weights are expected to deliver.

```typescript
export interface OptimisationSummary {
    /** The annual expected return. */
    expected_return: number;
    /** The annual volatility. */
    volatility: number;
    /** The Sharpe ratio, against the risk-free rate. */
    sharpe_ratio?: number | null;
    /**
     * The weighted average of the assets' volatilities over the portfolio's: how much
     * diversification reduces the risk. One for a single asset.
     */
    diversification_ratio?: number | null;
    /**
     * The number of equal weights with the same concentration: one over the sum of the squared
     * weights.
     */
    effective_assets: number;
    /** The asset with the largest weight. */
    largest_asset: string;
    /** Its weight. */
    largest_weight: number;
    /**
     * For minimum CVaR, the expected shortfall the weights reach, which the method minimises;
     * the shares of risk are then of it.
     */
    expected_shortfall?: ShortfallFigure | null;
    /**
     * For minimum semivariance, the downside deviation over one period the weights reach: the
     * square root of the semivariance below zero, which the method minimises.
     */
    downside_deviation?: number | null;
    /**
     * For minimum CDaR, the conditional drawdown at risk the weights reach, which the method
     * minimises.
     */
    drawdown_at_risk?: DrawdownFigure | null;
    /** The ex-ante tracking error a year against the benchmark weights, when they were given. */
    tracking_error?: number | null;
}
```

<a id="finalytics-optimiseoptions"></a>

### OptimiseOptions

The method, its parameters and the limits on the weights. Everything is optional: by default
the portfolio with the largest Sharpe ratio, fully invested and long only.

```typescript
export interface OptimiseOptions {
    /** The method: the largest Sharpe ratio by default. */
    method?: Method;
    /** The annual risk-free rate for the Sharpe ratio; by default the assets' own. */
    risk_free_rate?: number | null;
    /**
     * The annual expected return (`target_return`) or volatility (`target_volatility`) to aim
     * for, or the smallest annual expected return for minimum CVaR, semivariance and CDaR.
     */
    target?: number | null;
    /** The weight on variance, for mean–variance objectives. */
    risk_aversion?: number | null;
    /** The uncertainty of the expected returns, for robust mean–variance. */
    uncertainty?: ReturnUncertainty | null;
    /** The confidence of the conditional value at risk or drawdown at risk: 0.95 by default. */
    confidence?: number | null;
    /** The risk budgets, by asset, for risk budgeting; equal without them. */
    budgets?: Record<string, number> | null;
    /** How hierarchical risk parity joins clusters: single linkage by default. */
    linkage?: Linkage | null;
    /** The smallest and largest weight of every asset: 0 and 1 by default. */
    bounds?: Bound | null;
    /**
     * The largest weight of any asset, as `bounds` of 0 and `max_weight`; give one or the
     * other.
     */
    max_weight?: number | null;
    /** Bounds for particular assets, which take the place of `bounds` for them. */
    asset_bounds?: Record<string, Bound>;
    /** Limits on the total weight of groups of assets, by group name. */
    groups?: Record<string, AssetGroup>;
    /**
     * The smallest effective number of assets, `1 / sum w^2`, such as 5: the weights are
     * spread over at least as many assets as 5 equal weights would be.
     */
    min_effective_assets?: number | null;
    /**
     * Resampled optimisation (Michaud, 1998): the weights are the average of the optimal
     * weights for this many histories, such as 200, simulated from the estimates with as many
     * returns as the assets have, from random numbers with a fixed seed. For the methods that
     * use estimates, other than those measured on the periods' returns.
     */
    resample?: number | null;
    /** Annual expected returns by asset, in place of the estimates. */
    expected_returns?: Record<string, number>;
    /**
     * The weights held now, by asset, adding up to one with the cash: any of the assets, the
     * risk-free asset and cash in other currencies included, or `Cash` at the risk-free rate
     * where there is no risk-free asset. They are the start of the trades the report lists, and
     * of the turnover and trade limits; optimisation invests the cash it does not choose.
     */
    current_weights?: Record<string, number>;
    /**
     * The largest one-way turnover from the current weights, `sum |w - current| / 2`, such as
     * 0.2 for 20% of the portfolio bought (and as much sold).
     */
    max_turnover?: number | null;
    /**
     * The smallest trade, as a weight, such as 0.01: each asset is traded by at least this
     * much from its current weight, or not at all.
     */
    min_trade?: number | null;
    /** The largest number of assets traded from the current weights. */
    max_trades?: number | null;
    /**
     * A benchmark's weights by asset, adding up to one with the cash, as `current_weights`
     * may hold it: the report gives the weights' ex-ante tracking error against it, counting the
     * risk of any cash in other currencies.
     */
    benchmark_weights?: Record<string, number>;
    /** The largest ex-ante tracking error a year against `benchmark_weights`, such as 0.03. */
    max_tracking_error?: number | null;
    /**
     * The cash held now to keep at its current weight, as rebalancing keeps it: every
     * currency's (`true`) or that of the currencies named (`["USD"]`), at its weight in
     * `current_weights`. The rest of the portfolio is optimised, and the
     * limits are measured on the whole of it, so that rebalancing to the weights with the same
     * cash kept trades what the limits allow. The cash of a currency is the asset named for it,
     * such as `EUR cash`; `true` also keeps a risk-free asset that is not, such as T-bills, and
     * `Cash` at the risk-free rate. It needs `current_weights`, and the cash it keeps takes no
     * `cash` limits.
     */
    keep_cash?: KeepCash | null;
    /**
     * The smallest and largest weight of cash, such as 0 and 0.2: with it, optimisation may
     * hold cash beside the risky assets, riskless, and the frontier includes it. The cash is
     * the risk-free asset, earning its return, or, without one, an asset named `Cash` earning
     * the risk-free rate. Without it, the weights are fully invested in the risky assets.
     */
    cash?: Bound | null;
    /**
     * Whether the optimisation report draws the efficient frontier under the same limits.
     * Off by default: the frontier takes 30 more optimisations, so it adds time that grows with
     * the number of assets, to seconds for hundreds of assets. Only an optimisation reads it;
     * backtests leave it out.
     */
    frontier?: boolean;
}
```

<a id="finalytics-order"></a>

### Order

Which way matches are sorted.

```typescript
export type Order = "ascending" | "descending";
```

<a id="finalytics-outcome"></a>

### Outcome

The outcome of the plan, or of one of its tests.

```typescript
export interface Outcome {
    /** The chance of meeting the goal; absent without a goal. */
    chance?: number | null;
    /** The value at the end in the average market (the median), in today's money. */
    typical: number;
    /** For payments, the month by which half the simulations have run short, if they do. */
    runs_out?: Month | null;
}
```

<a id="finalytics-overviewreport"></a>

### OverviewReport

The overview report.

```typescript
export interface OverviewReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The headline figures. */
    headline: Headline;
    /** The portfolio's value on each valuation date. */
    value: Series;
    /** The growth of 100 at the time-weighted return. */
    growth: Series;
    /** The benchmark's growth of 100, with a benchmark. */
    benchmark_growth?: Series | null;
    /** The value split by holding and by each scheme of the options. */
    allocation: Breakdown[];
    /** The holdings' risk; absent (with a warning) when it cannot be estimated. */
    risk?: HoldingsRisk | null;
    /** Each holding's weight, expected return and risk. */
    holdings: HoldingRiskReturn[];
    /**
     * The efficient frontier of the instruments, long only and fully invested in them, from
     * the expected returns and the covariance: the frontier optimisation and the valuation of
     * weights draw, as neither chooses cash. Each point's weights are in the order of
     * `holdings`, with none in cash. Absent (with a warning) when it cannot be found.
     */
    frontier?: FrontierPoint[] | null;
    /** The current portfolio on the same axes as the frontier. */
    current?: PortfolioPoint | null;
    /**
     * The current portfolio with base-currency cash set aside: the other holdings in the shares
     * they have of each other, which the frontier, fully invested in them, can be compared
     * with. Cash in other currencies counts as a holding, as its value moves with the exchange
     * rate. The same as `current` when no base-currency cash is held.
     */
    current_without_base_cash?: PortfolioPoint | null;
}
```

<a id="finalytics-parameterkind"></a>

### ParameterKind

The kind of value an option takes.

```typescript
export type ParameterKind = "number" | "count" | "choice" | "by_asset" | "bound" | "bounds_by_asset" | "groups" | "currencies" | "uncertainty";
```

<a id="finalytics-paymentsoutcome"></a>

### PaymentsOutcome

What a goal of payments comes to.

```typescript
export interface PaymentsOutcome {
    /** The share of all the planned payments made, across the simulations. */
    paid: number;
    /** The share made in the simulations that run short; absent if none does. */
    paid_when_short?: number | null;
    /** The typical month the money runs short, among the simulations that run short. */
    runs_short?: Month | null;
    /** The month by which 1 simulation in 10 has run short: the weak market. */
    runs_out_weak?: Month | null;
    /** The month by which half the simulations have run short: the average market. */
    runs_out_average?: Month | null;
    /** The month by which 9 simulations in 10 have run short: the strong market. */
    runs_out_strong?: Month | null;
    /** The last month paid in full in at least the chance wanted of the simulations. */
    lasts_until?: Month | null;
}
```

<a id="finalytics-percentiles"></a>

### Percentiles

The 10th, 25th, 50th, 75th and 90th percentiles of a value across the simulations.

```typescript
export interface Percentiles {
    /** The 10th percentile: the weak market, 1 simulation in 10 below it. */
    p10: number;
    /** The 25th percentile. */
    p25: number;
    /** The median: the average market. */
    p50: number;
    /** The 75th percentile. */
    p75: number;
    /** The 90th percentile: the strong market, 1 simulation in 10 above it. */
    p90: number;
}
```

<a id="finalytics-performancereport"></a>

### PerformanceReport

The performance report.

```typescript
export interface PerformanceReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The portfolio's value on each valuation date. */
    value: Series;
    /** The growth of 100 at the time-weighted return. */
    growth: Series;
    /** The benchmark's growth of 100 over the same dates, with a benchmark. */
    benchmark_growth?: Series | null;
    /** The daily time-weighted returns. */
    returns: Series;
    /** Returns over MTD, QTD, YTD, one, three and five years, and since inception. */
    periods: PeriodRow[];
    /** Monthly returns: one row per year, one column per month and a last column for the year. */
    monthly: Grid;
    /** The benchmark's monthly returns, in the same layout, with a benchmark. */
    benchmark_monthly?: Grid | null;
    /** Calendar-year returns. */
    calendar_years: YearReturn[];
    /** Each holding's contribution to the return from the first valuation to the last. */
    contributions: Contribution[];
    /**
     * What fees and taxes charged to the accounts rather than to a holding took from the
     * return over the same period: negative, or zero without them. With the holdings'
     * contributions, it adds up to the time-weighted return.
     */
    expenses: number;
    /** Contributions by each scheme of the options. */
    groups: SchemeContributions[];
    /** The holdings that contributed most, largest first. */
    top_contributors: Contribution[];
    /** The holdings that took away most, most negative first. */
    top_detractors: Contribution[];
    /** External flows and income by month. */
    flows: MonthFlows[];
    /**
     * The household by account and by group of accounts, when the portfolio has more than one
     * account.
     */
    household?: Household | null;
}
```

<a id="finalytics-period"></a>

### Period

A reporting period ending on an as-of date.

```typescript
export type Period =
| {
    type: "month_to_date";
```

<a id="finalytics-periodattribution"></a>

### PeriodAttribution

The attribution of one period.

```typescript
export interface PeriodAttribution {
    /** The start of the period. */
    start: Date;
    /** The end of the period. */
    end: Date;
    /** The portfolio's return, `sum w r`. */
    portfolio_return: number;
    /** The benchmark's return, `sum b s`. */
    benchmark_return: number;
    /**
     * The coefficient by which the period's effects are multiplied before they are added up
     * over the periods (one for a single period).
     */
    linking_coefficient: number;
    /** Each segment's effects, in the order given. */
    segments: SegmentEffects[];
}
```

<a id="finalytics-periodreturn"></a>

### PeriodReturn

A return over a reporting period.

```typescript
export interface PeriodReturn {
    /** The valuation date the period starts from. */
    start: Date;
    /** The valuation date it ends on. */
    end: Date;
    /** The cumulative return over the period. */
    cumulative: number;
    /** The annualised return, given only for periods of a year or more. */
    annualised?: number | null;
}
```

<a id="finalytics-periodrow"></a>

### PeriodRow

Returns over one reporting period.

```typescript
export interface PeriodRow {
    /** The period. */
    period: Period;
    /** Its short name: "MTD", "QTD", "YTD", "1Y", "3Y", "5Y" or "ITD". */
    label: string;
    /**
     * The valuation the period starts from, absent when the history does not reach back to
     * the period's start (the other figures are then absent too).
     */
    start?: Date | null;
    /** The valuation it ends on. */
    end?: Date | null;
    /** The cumulative time-weighted return. */
    cumulative?: number | null;
    /** The annualised time-weighted return, for periods of a year or more. */
    annualised?: number | null;
    /**
     * The money-weighted return: the internal rate of return, annualised for periods of a year
     * or more and over the period itself for shorter ones, as the time-weighted return is.
     */
    money_weighted?: number | null;
    /** The Modified Dietz return. */
    modified_dietz?: number | null;
    /** The benchmark's return over the same dates. */
    benchmark?: number | null;
    /** `cumulative - benchmark`. */
    excess_arithmetic?: number | null;
    /** `(1 + cumulative) / (1 + benchmark) - 1`. */
    excess_geometric?: number | null;
}
```

<a id="finalytics-plan"></a>

### Plan

A plan: the planning report and the raw results of its simulations. Printing it prints the
report, and showing it shows the report as a page.

```typescript
export interface Plan {
    /** The planning report. */
    report: PlanReport;
    /**
     * The raw results: the chance, the value over time, the changes, the tests when markets do
     * worse and the alternatives.
     */
    results: PlanResults;
}
```

<a id="finalytics-plancontext"></a>

### PlanContext

What a plan simulated, beside what `PlanResults` records: what the report needs to
describe it.

```typescript
export interface PlanContext {
    /** The engine's version. */
    version: string;
    /**
     * A SHA-256 digest of the assets' returns and the plan, in hexadecimal: the same inputs
     * give the same report.
     */
    digest: string;
    /** The goal; absent for a projection. */
    goal?: Goal | null;
    /** The value at the start, in today's money. */
    start_value: number;
    /** The plan's investment mix. */
    mix: Mix;
    /** The mix's name, such as "Balanced 60/40", or "Your mix" when it has none. */
    mix_name: string;
    /** The assets the simulations draw from, in the order of the history. */
    assets: string[];
    /** The first month of the history of returns. */
    history_start: Month;
    /** The last month of the history of returns. */
    history_end: Month;
    /** How markets were simulated. */
    model: Model;
    /** The expected returns a year given for the assets, if any. */
    expected_returns?: Record<string, number> | null;
    /** How amounts are kept in today's money. */
    inflation: InflationBasis;
    /** The fee a year, such as 0.0025. */
    fee: number;
    /** The currency of the amounts, if given. */
    currency?: Currency | null;
    /** The seed of the random draws. */
    seed: number;
}
```

<a id="finalytics-planflow"></a>

### PlanFlow

Money paid into the plan, or taken out of it, other than the goal: once, in the month `on`,
or regularly, `every` month, quarter or year from `start` to `end`.

```typescript
export type PlanFlow =
| {
    type: "saving";
```

<a id="finalytics-planoptions"></a>

### PlanOptions

What a plan simulates and how.

```typescript
export interface PlanOptions {
    /** The goal; without one, the plan is a projection to `end`. */
    goal?: Goal | null;
    /** The chance of meeting the goal wanted, such as 0.8. */
    chance?: number;
    /**
     * The chance from which the plan has room to spare: 10 points above `chance` by default,
     * at most 0.99.
     */
    spare?: number | null;
    /** The first month: the month after the assets' last date by default. */
    start?: Month | null;
    /** The last month of a projection without a goal. */
    end?: Month | null;
    /** What the money set aside is worth at the start, in today's money. */
    start_value?: number;
    /** Money paid in and taken out, other than the goal. */
    flows?: PlanFlow[];
    /** What the money is invested in. */
    weights?: Mix | null;
    /** Other mixes to compare on the same simulations, each named. */
    alternatives?: Mix[];
    /** The fee a year, as a share of the value, taken monthly. */
    fee?: number;
    /** How prices rise: required, as a rate or an index. */
    inflation?: Inflation | null;
    /** How each month's returns are drawn: a block bootstrap of 12 months by default. */
    model?: Model;
    /**
     * Expected returns a year by asset, in place of the history's: they shift the bootstrap's
     * returns and set the normal model's means.
     */
    expected_returns?: Record<string, number> | null;
    /** The number of simulations: 10,000 by default. */
    simulations?: number;
    /** The seed of the random numbers: the same seed gives the same plan on every platform. */
    seed?: number;
    /** The currency of the amounts, for the report. */
    currency?: Currency | null;
    /** The report's title: the goal's name by default. */
    name?: string | null;
}
```

<a id="finalytics-planreport"></a>

### PlanReport

The planning report: what a plan's simulations show, with the plan it simulated.

```typescript
export interface PlanReport {
    /** What was simulated and how. */
    context: PlanContext;
    /** What the simulations show. */
    figures: PlanResults;
    /**
     * The report's title, such as "Retirement income": shown when it is printed or laid out
     * as a page.
     */
    name?: string | null;
}
```

<a id="finalytics-planresults"></a>

### PlanResults

What a plan's simulations show: the chance of the goal, the value over time, what would
change the outcome, the plan when markets do worse, and the alternatives.

```typescript
export interface PlanResults {
    /** The first month. */
    start: Month;
    /**
     * The month after the last one simulated: a target's date, or the month after the last
     * payment or the end of a projection.
     */
    end: Month;
    /** The number of simulations. */
    simulations: number;
    /** The chance wanted. */
    chance_wanted: number;
    /** The chance from which there is room to spare. */
    spare_from: number;
    /** The status; absent without a goal. */
    status?: Status | null;
    /** The plan's outcome. */
    outcome: Outcome;
    /** The months recorded: every month, or every few months for a long plan. */
    months: Month[];
    /** The value at the start of each recorded month across the simulations. */
    values: Percentiles[];
    /** The value at the end across the simulations. */
    end_value: Percentiles;
    /** For a target, the typical amount short among the simulations that miss it. */
    shortfall?: number | null;
    /** For payments, how many are made and when the money runs short. */
    payments?: PaymentsOutcome | null;
    /**
     * For a target, the chance of having the amount by each month from the start to the
     * target's date, saving as planned.
     */
    chance_by_month?: number[];
    /**
     * 100 simulations spread evenly from the best outcome to the worst, so that the share of
     * them that meets the goal is the chance.
     */
    sample: SamplePath[];
    /** The flows as simulated, in the order given. */
    flows: FlowSummary[];
    /** What would change the outcome; absent without a goal. */
    changes?: Changes | null;
    /** The plan when markets do worse. */
    what_if: WhatIf;
    /** The risk of the plan's mix. */
    risk: MixRisk;
    /** The plan's mix at the start, by asset. */
    weights: [string, number][];
    /** The alternatives, in the order given. */
    alternatives: MixOutcome[];
}
```

<a id="finalytics-portfoliodata"></a>

### PortfolioData

A portfolio's records: its base currency, accounts, instruments and transactions. With market
data, they are everything Finalytics needs to keep the portfolio's books.

```typescript
export interface PortfolioData {
    /** The currency results are reported in. */
    base_currency: Currency;
    /** The accounts. */
    accounts: Account[];
    /** The instruments the transactions refer to. */
    instruments: Instrument[];
    /**
     * The transactions, in any order; they are applied by trade date, and in the order given
     * within a date.
     */
    transactions: Transaction[];
}
```

<a id="finalytics-portfoliodocument"></a>

### PortfolioDocument

A portfolio and, optionally, the market data to value it, in one versioned document.

```typescript
export interface PortfolioDocument {
    /** Always `"finalytics.portfolio"`. */
    format: "finalytics.portfolio";
    /** The version of the format, 1 for this release. */
    format_version: 1;
    /** The portfolio. */
    portfolio: PortfolioData;
    /** Market data stored with the portfolio. */
    market?: DocumentMarketData | null;
    /** Goals kept with the portfolio, each a plan with its name, in the order given. */
    goals?: PlanOptions[];
}
```

<a id="finalytics-portfoliopoint"></a>

### PortfolioPoint

A portfolio's expected return and volatility, for a point on a risk–return chart.

```typescript
export interface PortfolioPoint {
    /** The annual expected return. */
    expected_return: number;
    /** The annualised volatility. */
    volatility: number;
}
```

<a id="finalytics-portfolioreport"></a>

### PortfolioReport

The six reports on a portfolio, from one set of options. The attribution report needs the
benchmark's segments, and the scenarios report needs scenarios, in the options; without
them, they are absent.

```typescript
export interface PortfolioReport {
    /** Value, returns and risk at a glance, with the efficient frontier. */
    overview: OverviewReport;
    /** Holdings, lots, income and realised gains. */
    holdings: HoldingsReport;
    /** Returns over periods, contribution and flows. */
    performance: PerformanceReport;
    /** Volatility, tail risk, drawdowns and where the risk comes from. */
    risk: RiskReport;
    /** Attribution against the benchmark. */
    attribution?: AttributionReport | null;
    /** The effect of scenarios on value and risk. */
    scenarios?: ScenariosReport | null;
    /**
     * The portfolio's name, shown under each report's title when it is printed or laid out as
     * a page.
     */
    name?: string | null;
}
```

<a id="finalytics-position"></a>

### Position

A position: the open lots of one instrument in one account, added up.

```typescript
export interface Position {
    /** The account. */
    account: AccountId;
    /** The instrument. */
    instrument: InstrumentId;
    /** The total quantity. */
    quantity: Decimal;
    /** The total cost basis in the instrument's currency. */
    cost_local: Decimal;
    /** The total cost basis in the base currency. */
    cost_base: Decimal;
    /** The number of open lots. */
    lots: number;
}
```

<a id="finalytics-positionvalue"></a>

### PositionValue

A position valued on a date.

```typescript
export interface PositionValue {
    /** The account. */
    account: AccountId;
    /** The instrument. */
    instrument: InstrumentId;
    /** The quantity held. */
    quantity: Decimal;
    /** The price used, per unit, in the instrument's currency. */
    price: Decimal;
    /** The date the price was observed (earlier than the valuation date if carried forward). */
    price_date: Date;
    /** `quantity × price × multiplier`, in the instrument's currency. */
    market_value_local: Decimal;
    /** Units of base currency per unit of the instrument's currency. */
    fx_rate: Decimal;
    /** The market value in the base currency. */
    market_value_base: Decimal;
    /** The cost basis in the instrument's currency. */
    cost_local: Decimal;
    /** The cost basis in the base currency. */
    cost_base: Decimal;
    /** `market_value_local - cost_local`. */
    unrealised_local: Decimal;
    /** `market_value_base - cost_base`, which includes the currency effect. */
    unrealised_base: Decimal;
}
```

<a id="finalytics-pricetable"></a>

### PriceTable

The prices of several assets on the same dates, one column per asset. A missing price is
written as absent (`null`).

```typescript
export interface PriceTable {
    /** The dates, in increasing order. */
    dates: Date[];
    /** The assets, one per column. */
    labels: string[];
    /** The prices of each asset, one per date; absent where there is no price. */
    columns: ((number | null)[])[];
}
```

<a id="finalytics-prior"></a>

### Prior

The prior expected returns of the Black–Litterman model.

```typescript
export type Prior =
| {
    type: "market";
```

<a id="finalytics-problem"></a>

### Problem

A portfolio optimisation problem: the assets, estimates of their risk and return, and the
constraints.

Risk comes from a covariance matrix or from a factor model, which the solver uses in factor
form, so that large problems solve faster. The risk estimates and expected returns should be
in the same units, for example both annualised. Scenario returns, for minimum CVaR, are per
period.

```typescript
export interface Problem {
    /** The asset labels. */
    labels: string[];
    /** The covariance matrix of the assets' returns. */
    covariance?: Matrix | null;
    /**
     * A factor model of the assets' returns, in place of a covariance matrix. Its assets must
     * be the problem's labels, in order.
     */
    factor_model?: FactorModel | null;
    /** The expected return of each asset. */
    expected_returns?: number[] | null;
    /** Scenario returns, one column per asset, for minimum CVaR. */
    scenarios?: ReturnsTable | null;
    /** The constraints. */
    constraints?: Constraints;
}
```

<a id="finalytics-proposedconversion"></a>

### ProposedConversion

A proposed currency conversion.

```typescript
export interface ProposedConversion {
    /** The amount sold. */
    sell: Money;
    /** The amount bought. */
    buy: Money;
}
```

<a id="finalytics-proposedtrade"></a>

### ProposedTrade

A proposed trade in one instrument.

```typescript
export interface ProposedTrade {
    /** The instrument. */
    instrument: InstrumentId;
    /** Buy or sell. */
    side: Side;
    /** The units to trade, greater than zero. */
    quantity: Decimal;
    /** The price per unit, in the instrument's currency. */
    price: Decimal;
    /** The instrument's currency. */
    currency: Currency;
    /** `quantity × price × multiplier`, in the instrument's currency. */
    value_local: Decimal;
    /** The value in the base currency. */
    value_base: Decimal;
    /** For a sale, the lots it closes. */
    lots: LotSale[];
    /** The fee, in the instrument's currency. */
    fees_local: Decimal;
    /**
     * For a sale, the gain it would realise in the base currency, after fees, as the books
     * would record it.
     */
    realised_gain_base?: Decimal | null;
    /**
     * For a sale with tax rates, the estimated tax on its gains in the base currency; negative
     * when its losses save tax. None in an account that is not taxable.
     */
    tax_base?: Decimal | null;
    /**
     * For a sale with tax rates, the part of its realised gain from lots held long-term; the
     * rest is short-term.
     */
    long_term_gain_base?: Decimal | null;
}
```

<a id="finalytics-provenance"></a>

### Provenance

Where a report's numbers come from, so that a saved report can be checked to reproduce.

```typescript
export interface Provenance {
    /** Which report this is. */
    report: ReportKind;
    /** The engine: "finalytics". */
    engine: string;
    /** The engine's version. */
    version: string;
    /** The date of the report. */
    as_of: Date;
    /** The first valuation date. */
    start: Date;
    /** The currency of every amount. */
    base_currency: Currency;
    /** The inputs. */
    source: Source;
    /** The options the report was computed with. */
    options: ReportOptions;
    /**
     * Prices and rates carried forward from earlier dates, and figures that could not be
     * computed.
     */
    warnings: ValidationReport;
    /**
     * The benchmark, as the reports describe it, such as "BENCH, in USD, converted to EUR at
     * the market data's exchange rates".
     */
    benchmark?: string | null;
    /**
     * The accounts the report covers, when not every one, as the reports describe them, such
     * as "Brokerage only (1 of 2 accounts). 3 transfers with Retirement count as money in and
     * out."
     */
    accounts?: string | null;
}
```

<a id="finalytics-realisedgain"></a>

### RealisedGain

A gain or loss realised by a sale, a return of capital or a cash merger, for one lot.

```typescript
export interface RealisedGain {
    /** The transaction that realised it. */
    transaction: TransactionId;
    /** The account. */
    account: AccountId;
    /** The instrument. */
    instrument: InstrumentId;
    /** The lot. */
    lot: LotId;
    /** The units disposed of (zero for a return of capital beyond the cost basis). */
    quantity: Decimal;
    /** When the lot was acquired. */
    acquired: Date;
    /** When it was disposed of. */
    disposed: Date;
    /** Proceeds net of fees, in the instrument's currency. */
    proceeds_local: Decimal;
    /** Cost basis in the instrument's currency. */
    cost_local: Decimal;
    /** `proceeds_local - cost_local`. */
    gain_local: Decimal;
    /** Proceeds in the base currency, at the disposal date's rate. */
    proceeds_base: Decimal;
    /** Cost basis in the base currency, at the acquisition rates. */
    cost_base: Decimal;
    /** `proceeds_base - cost_base`, which includes the currency effect. */
    gain_base: Decimal;
}
```

<a id="finalytics-realisedsummary"></a>

### RealisedSummary

Gains realised up to the report's date, in the base currency.

```typescript
export interface RealisedSummary {
    /** Every realised gain. */
    total: Decimal;
    /** Gains on lots held for the long-term period or less. */
    short_term: Decimal;
    /** Gains on lots held for longer than the long-term period. */
    long_term: Decimal;
    /** By calendar year of disposal. */
    by_year: Record<string, Decimal>;
    /** By instrument. */
    by_instrument: Record<string, Decimal>;
}
```

<a id="finalytics-rebalance"></a>

### Rebalance

The trades that move an account to target weights, and their effects.

```typescript
export interface Rebalance {
    /** The account. */
    account: AccountId;
    /** The date of the prices used. */
    date: Date;
    /** The account's value in the base currency. */
    value_base: Decimal;
    /** Sales, then purchases. */
    trades: ProposedTrade[];
    /** Conversions that fund purchases in other currencies and reach the target cash balances. */
    conversions: ProposedConversion[];
    /** Each holding's weight before the trades. */
    weights_before: Record<string, number>;
    /** Each holding's weight after the trades. */
    weights_after: Record<string, number>;
    /**
     * The one-way turnover: half the sum of the changes in weight, cash included, as
     * optimisation counts it and its `max_turnover` limits.
     */
    turnover: number;
    /** The gain the sales would realise in the base currency, after fees. */
    realised_gain_base: Decimal;
    /** The fees of all the trades, in the base currency. */
    fees_base: Decimal;
    /**
     * With tax rates, the estimated tax on the sales' gains in the base currency; None in an
     * account that is not taxable.
     */
    tax_base?: Decimal | null;
    /** Whether the account's gains are taxed when they are realised. */
    taxable?: boolean;
    /** Carried prices, a shortfall of cash, and trades left out by rounding. */
    warnings: ValidationReport;
}
```

<a id="finalytics-rebalancecontext"></a>

### RebalanceContext

What a rebalance report describes: the date, the targets, the accounts and the choices.

```typescript
export interface RebalanceContext {
    /** The engine's version. */
    version: string;
    /** A SHA-256 digest of the targets, the choices and the proposal. */
    digest: string;
    /** The date of the prices. */
    date: Date;
    /** The currency of the values. */
    base_currency: Currency;
    /**
     * The method of the optimisation the targets came from, such as "Minimum variance"; absent
     * for weights given by holding.
     */
    method?: string | null;
    /** That optimisation's limits in words, such as "5% to 40% in each asset". */
    limits: string[];
    /** Whether trades are in whole units. */
    whole_units: boolean;
    /**
     * How sales choose lots when no tax rates are given: absent for each account's own
     * method.
     */
    lot_method?: LotMethod | null;
    /** The smallest trade, in the base currency. */
    minimum_trade: number;
    /** The fees each trade pays. */
    fees: Fees;
    /** The tax rates on realised gains, if given. */
    tax?: TaxRates | null;
    /**
     * The estimates behind the volatility and expected return before and after; absent when
     * the holdings' returns could not be estimated.
     */
    estimates?: RebalanceEstimates | null;
    /** The cash kept as it was, by currency, as asked. */
    kept_cash?: KeptCash[];
}
```

<a id="finalytics-rebalanceestimates"></a>

### RebalanceEstimates

The estimates behind a rebalance's volatility and expected return.

```typescript
export interface RebalanceEstimates {
    /** The covariance estimator. */
    covariance: CovarianceMethod;
    /** Return periods in a year. */
    periods_per_year: number;
    /** The first date of the returns. */
    start: Date;
    /** The last date of the returns. */
    end: Date;
}
```

<a id="finalytics-rebalanceoptions"></a>

### RebalanceOptions

Choices for `rebalance`.

```typescript
export interface RebalanceOptions {
    /**
     * How sales choose lots: by default, each account's own method (first in, first out
     * unless the account says otherwise). Highest cost first defers gains. Ignored when `tax`
     * is given.
     */
    lot_method?: LotMethod | null;
    /** Trade whole units only, rounding each trade towards zero. */
    whole_units?: boolean;
    /** Leave out trades worth less than this in the base currency. */
    minimum_trade?: Decimal;
    /** The fees each trade pays; none by default. */
    fees?: Fees;
    /**
     * Tax rates on realised gains. When given, each sale takes the lots that cost the least tax,
     * losses first, and the tax is estimated.
     */
    tax?: TaxRates | null;
    /**
     * Currencies whose cash the rebalancing keeps as it is: the other targets share the rest of
     * the value, so the trades neither invest that cash nor add to it. Targets that leave the
     * kept cash out are shares of the rest; targets that give it its weight now are shares of
     * the whole value. None by default: the targets are shares of the whole value, and
     * base-currency cash takes what they leave.
     */
    keep_cash?: Currency[];
}
```

<a id="finalytics-rebalancereport"></a>

### RebalanceReport

The trades that move a portfolio's accounts to target weights: how each holding's weight
moves, what the trades change and cost, each account's trades, and what to check before
making them.

```typescript
export interface RebalanceReport {
    /** What was rebalanced, to what, and how. */
    context: RebalanceContext;
    /** The trades and their costs, and what they change. */
    summary: RebalanceSummary;
    /**
     * Each holding's weight before the trades, at the target and after, in the accounts
     * together: the largest target first, then cash.
     */
    holdings: HoldingMove[];
    /** Each account's trades, conversions, costs and warnings. */
    accounts: AccountTrades[];
    /**
     * The report's title, such as "Morgan family portfolio": shown when it is printed or laid
     * out as a page.
     */
    name?: string | null;
}
```

<a id="finalytics-rebalanceresults"></a>

### RebalanceResults

The trades that move several accounts together to target weights of their combined value,
one proposal per account.

```typescript
export interface RebalanceResults {
    /** The proposal for each account with value, in the order given. */
    accounts: Rebalance[];
    /** The base currency, of the values below. */
    base_currency: Currency;
    /** The accounts' combined value in the base currency. */
    value_base: Decimal;
    /** Each holding's weight in the combined value before the trades. */
    weights_before: Record<string, number>;
    /** Each holding's weight in the combined value after the trades. */
    weights_after: Record<string, number>;
    /**
     * The one-way turnover of all the accounts: half the sum of the changes in each holding's
     * weight in their combined value, cash included, as optimisation counts it and its
     * `max_turnover` limits.
     */
    turnover: number;
    /** The gain the sales would realise in the base currency, after fees. */
    realised_gain_base: Decimal;
    /** The fees of all the trades, in the base currency. */
    fees_base: Decimal;
    /** With tax rates, the estimated tax on the sales' gains in the base currency. */
    tax_base?: Decimal | null;
    /** The cash kept as it was, by currency, as `keep_cash` asked. */
    kept_cash?: KeptCash[];
}
```

<a id="finalytics-rebalanceschedule"></a>

### RebalanceSchedule

When the portfolio trades back to its target weights. Between trades the weights drift with
prices.

```typescript
export type RebalanceSchedule =
| {
    type: "monthly";
```

<a id="finalytics-rebalancesummary"></a>

### RebalanceSummary

The trades of a rebalance in figures: how many, how much is traded, what they cost and what
they change.

```typescript
export interface RebalanceSummary {
    /** The accounts' combined value. */
    value_base: number;
    /** The number of trades. */
    trades: number;
    /** The number of sales. */
    sales: number;
    /** The number of purchases. */
    purchases: number;
    /**
     * The one-way turnover: half the changes in each holding's weight in the combined value,
     * cash included.
     */
    turnover: number;
    /** The value traded. */
    traded_base: number;
    /** The fees of all the trades. */
    fees_base: number;
    /** The gain the sales realise, after fees. */
    realised_gain_base: number;
    /** With tax rates, the part of the realised gain from lots held long-term. */
    long_term_gain_base?: number | null;
    /** With tax rates, the estimated tax on the sales in taxable accounts. */
    tax_base?: number | null;
    /** The unrealised gain of the accounts before the trades. */
    unrealised_gain_base: number;
    /** Volatility and expected return a year before the trades. */
    before?: RiskReturn | null;
    /** After the trades. */
    after?: RiskReturn | null;
    /** At the targets. */
    target?: RiskReturn | null;
}
```

<a id="finalytics-rebalancing"></a>

### Rebalancing

A rebalancing: the rebalance report on the trades, and the raw results, each account's
proposal. Printing it prints the report, showing it shows the report as a page, and the
portfolio's `with_rebalance` makes the trades.

```typescript
export interface Rebalancing {
    /**
     * The rebalance report: how the weights move, what the trades change and cost, each
     * account's trades, what to check, and how the trades were found.
     */
    report: RebalanceReport;
    /** The raw results: each account's trades, conversions, weights and costs. */
    results: RebalanceResults;
}
```

<a id="finalytics-relativestatistics"></a>

### RelativeStatistics

Statistics of a returns series against a benchmark's returns over the same periods.

```typescript
export interface RelativeStatistics {
    /** The number of returns. */
    observations: number;
    /** The annualised return less the benchmark's annualised return. */
    active_return: number;
    /** The annualised standard deviation of the active returns (returns less benchmark returns). */
    tracking_error: number;
    /** The annualised mean active return over the tracking error. */
    information_ratio?: number | null;
    /** The covariance of the returns with the benchmark's over the benchmark's variance. */
    beta?: number | null;
    /**
     * The mean return not explained by beta, in excess of the risk-free rate, annualised by
     * compounding: `(1 + mean(r - rf - beta (b - rf)))^periods_per_year - 1`.
     */
    alpha?: number | null;
    /** The correlation of the returns with the benchmark's. */
    correlation?: number | null;
    /**
     * The beta over the periods the benchmark fell: the slope of the returns on the
     * benchmark's in those periods alone. Absent with fewer than two such periods.
     */
    downside_beta?: number | null;
    /** The beta over the periods the benchmark rose. */
    upside_beta?: number | null;
    /**
     * The compound annual return over the periods the benchmark fell, over the benchmark's in
     * those periods: below 1, the portfolio lost less than the benchmark when it fell.
     */
    downside_capture?: number | null;
    /** The same over the periods the benchmark rose: above 1, the portfolio gained more. */
    upside_capture?: number | null;
}
```

<a id="finalytics-reportbenchmark"></a>

### ReportBenchmark

The benchmark of the portfolio reports.

An instrument's levels are in its currency, as the portfolio's instruments give it (the base
currency for one they do not list), and are converted to the base currency at the market
data's rates. A blend holds its instruments, and any cash, at the weights given, rebalanced to
them at the start of each month, each in the base currency in the same way; the attribution
report derives the benchmark's segments from it.

```typescript
export type ReportBenchmark = InstrumentId | Record<string, number>;
```

<a id="finalytics-reportkind"></a>

### ReportKind

Which report a report is.

```typescript
export type ReportKind = "holdings" | "overview" | "performance" | "risk" | "attribution" | "scenarios";
```

<a id="finalytics-reportoptions"></a>

### ReportOptions

The options every report takes. Each report uses the ones that concern it; the others are
ignored, so one set of options can serve every screen.

```typescript
export interface ReportOptions {
    /** The date of the report: the last price date by default. */
    as_of?: Date | null;
    /** The first valuation date: the first transaction's date by default. */
    start?: Date | null;
    /**
     * The accounts to report on, by identifier: every account by default. The figures are
     * those of the books kept from their transactions alone, so transfers with the accounts
     * left out count as money in and out, and each report says how many there are.
     */
    accounts?: AccountId[];
    /**
     * Classification schemes of the instruments to break holdings, contribution and risk down
     * by, such as `asset_class` or `region`.
     */
    group_by?: string[];
    /**
     * The benchmark: an instrument whose prices in the market data are its levels, such as an
     * index, which need not be held; or a blend of instruments by weight, such as
     * `{"VTI": 0.6, "BND": 0.4}`. Either is in the base currency, converted at the market
     * data's exchange rates.
     */
    benchmark?: ReportBenchmark | null;
    /** The annual risk-free rate, for Sharpe ratios. The default is 0. */
    risk_free_rate?: number;
    /** Valuation periods in a year, for annualising: 252 (trading days) by default. */
    periods_per_year?: number;
    /** When external flows happen within the day: at the start (the default) or the end. */
    timing?: FlowTiming;
    /** How the covariance of the holdings' returns is estimated: Ledoit–Wolf by default. */
    covariance?: CovarianceMethod;
    /** The confidences of value at risk and expected shortfall: 0.95 and 0.99 by default. */
    confidences?: number[];
    /** The window of rolling statistics, in valuation periods: 63 by default. */
    rolling_window?: number;
    /** The number of bins of return histograms: 30 by default. */
    histogram_bins?: number;
    /**
     * How many holdings lists of the largest, the top contributors and the like show: 10 by
     * default.
     */
    top?: number;
    /**
     * The holding period, in days, after which a lot or a gain is long-term: a lot held for
     * more than this many days is long-term, as in rebalancing's tax rates. 365 by default.
     */
    long_term_after_days?: number;
    /**
     * Annual expected returns by instrument for the efficient frontier; instruments not listed,
     * and every instrument when none is, take their historical mean return.
     */
    expected_returns?: Record<string, number>;
    /** The number of portfolios on the efficient frontier: 25 by default. */
    frontier_points?: number;
    /** Benchmark weights by holding, for the ex-ante tracking error. */
    benchmark_weights?: Record<string, number>;
    /** The benchmark's segments, for the attribution report. */
    attribution?: AttributionInputs | null;
    /** The scenarios of the scenarios report. */
    scenarios?: NamedScenario[];
}
```

<a id="finalytics-reporttable"></a>

### ReportTable

A table of a report: named, with typed columns and one row of cells per record.

```typescript
export interface ReportTable {
    /** A stable name, such as `holdings`, usable as a file or sheet name. */
    name: string;
    /** A title for people, such as "Holdings". */
    title: string;
    /** The columns. */
    columns: Column[];
    /** The rows, one cell per column. */
    rows: Cell[][];
}
```

<a id="finalytics-resampledsolution"></a>

### ResampledSolution

The result of resampled optimisation.

```typescript
export interface ResampledSolution {
    /** The objective each simulated problem was solved for. */
    objective: Objective;
    /** The asset labels. */
    labels: string[];
    /** The average of the simulated problems' optimal weights, absent if none was solved. */
    weights?: number[] | null;
    /**
     * The standard deviation of each weight across the simulated problems (divisor `m - 1` for
     * `m` solved problems), absent with fewer than two.
     */
    weight_deviations?: number[] | null;
    /** The expected return of the average weights under the problem's own estimates. */
    expected_return?: number | null;
    /** The volatility of the average weights under the problem's own estimates. */
    volatility?: number | null;
    /** The largest amount by which the average weights break the problem's constraints. */
    max_violation?: number | null;
    /** The number of simulated problems solved and averaged. */
    solved: number;
    /** The number of simulated problems the solver could not solve, left out of the average. */
    failed: number;
    /** The time taken, in seconds. It varies from run to run and is left out of comparisons. */
    solve_time: number;
}
```

<a id="finalytics-resampling"></a>

### Resampling

How to resample a problem's estimates.

```typescript
export interface Resampling {
    /** The number of simulated histories, at least 2; 100 to 500 is usual. */
    samples: number;
    /**
     * The number of periods in each simulated history, at least 2: the length of the history
     * the estimates came from, so that the simulated estimates are as uncertain as the real
     * ones.
     */
    observations: number;
    /**
     * The seed of the random numbers. The same seed gives the same result on every platform and
     * with any number of threads.
     */
    seed: number;
    /**
     * For estimates annualised from returns with this many periods in a year (12 for
     * monthly), the histories are simulated period by period, from the estimates divided by
     * it, and their estimates annualised again, so that they are as uncertain as estimates
     * from `observations` periods. Without it, each simulated period is in the estimates' own
     * units.
     */
    periods_per_year?: number | null;
}
```

<a id="finalytics-resamplingsummary"></a>

### ResamplingSummary

How resampled weights were found: the average of the optimal weights for histories simulated
from the estimates (Michaud, 1998).

```typescript
export interface ResamplingSummary {
    /** The number of histories simulated. */
    samples: number;
    /** The number of periods in each, as many as the returns the estimates came from. */
    observations: number;
    /** The number of simulated problems solved and averaged. */
    solved: number;
}
```

<a id="finalytics-returnuncertainty"></a>

### ReturnUncertainty

Where the true expected returns may lie around the estimates, for robust mean–variance
optimisation.

```typescript
export type ReturnUncertainty =
| {
    type: "ellipsoid";
```

<a id="finalytics-returnstable"></a>

### ReturnsTable

The returns of several assets over the same periods, one column per asset.

```typescript
export interface ReturnsTable {
    labels: string[];
    columns: number[][];
}
```

<a id="finalytics-riskcontribution"></a>

### RiskContribution

One asset's share of a portfolio's risk: its volatility, or its expected shortfall.

```typescript
export interface RiskContribution {
    /** The asset, as labelled in the covariance matrix. */
    label: string;
    /** Its weight. */
    weight: number;
    /**
     * The change in the portfolio's risk per unit of weight: for volatility,
     * `(C w)_i / volatility`; for expected shortfall, the asset's mean loss in the tail
     * scenarios. Absent when the portfolio has no volatility.
     */
    marginal?: number | null;
    /** `weight * marginal`. The components add up to the portfolio's risk. */
    component: number;
    /**
     * `component` over the portfolio's risk. The shares add up to one. Absent when the
     * portfolio has no risk.
     */
    share?: number | null;
}
```

<a id="finalytics-riskdecomposition"></a>

### RiskDecomposition

A portfolio's volatility split into each asset's contribution (Euler allocation).

```typescript
export interface RiskDecomposition {
    /**
     * The portfolio's volatility, `sqrt(w' C w)`, in the units of the covariance matrix (scale
     * the matrix to annualise).
     */
    volatility: number;
    /** Each asset's contribution, in the order of the covariance matrix. */
    contributions: RiskContribution[];
}
```

<a id="finalytics-riskreport"></a>

### RiskReport

The risk report.

```typescript
export interface RiskReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The portfolio's value on the report's date. */
    value: number;
    /** The portfolio's time-weighted returns, one per valuation period. */
    returns: Series;
    /** Statistics of the returns, annualised. */
    statistics: Statistics;
    /** Statistics against the benchmark, with a benchmark. */
    relative?: RelativeStatistics | null;
    /** The fall from the running peak on each valuation date. */
    drawdown: Series;
    /** The largest drawdowns, deepest first. */
    drawdowns: Drawdown[];
    /**
     * Value at risk and expected shortfall by every method at every confidence of the
     * options.
     */
    tail: TailRow[];
    /** The distribution of the returns, with the normal distribution's for comparison. */
    histogram: Histogram;
    /** The current holdings' risk; absent (with a warning) when it cannot be estimated. */
    holdings?: HoldingsRisk | null;
    /**
     * Rolling annualised volatility, Sharpe ratio and, with a benchmark, beta and tracking
     * error.
     */
    rolling: Series[];
}
```

<a id="finalytics-riskreturn"></a>

### RiskReturn

A portfolio's volatility and expected return, a year.

```typescript
export interface RiskReturn {
    /** The annual volatility. */
    volatility: number;
    /** The annual expected return. */
    expected_return: number;
}
```

<a id="finalytics-rule"></a>

### Rule

A rule every match must meet: a measure or a numeric field compared with a number (`<`,
`<=`, `>`, `>=`) or a range (`between`), or a text field that is (`in`) or is not (`not_in`)
one of a list of values. A security without the value a rule tests fails it.

```typescript
export interface Rule {
    /** The measure the rule tests, such as `"return_1y"`; or give `field`. */
    measure?: Measure | null;
    /** The field the rule tests, such as `"sector"` or `"pe_ratio"`; or give `measure`. */
    field?: string | null;
    /** How the value is compared. */
    op: Op;
    /** The number compared with, or `[low, high]` for `between`. */
    value?: RuleValue | null;
    /** The text values of `in` and `not_in`. */
    values?: string[] | null;
}
```

<a id="finalytics-ruleresult"></a>

### RuleResult

A rule, with how many securities met it.

```typescript
export interface RuleResult {
    /** The rule. */
    rule: Rule;
    /** How many securities that are not stale meet it. */
    passed: number;
    /** How many that are not stale have no value of what it tests, and so fail it. */
    lacking: number;
    /** How many meet it and every rule before it. */
    remaining: number;
}
```

<a id="finalytics-rulevalue"></a>

### RuleValue

The number a numeric rule compares with, or the `[low, high]` range of `between`.

```typescript
export type RuleValue = number | number[];
```

<a id="finalytics-run"></a>

### Run

A strategy held over a run of periods, rebalanced on the schedule, net of trading costs.

```typescript
export interface Run {
    /** The date each period ends on. */
    dates: Date[];
    /**
     * The return of each period, net of the cost of the trades made at the end of the one
     * before.
     */
    returns: number[];
    /**
     * The weights bought at the start, by asset; what they leave is held in cash. The start
     * costs nothing.
     */
    start: Record<string, number>;
    /** Every trade after the start. */
    trades: Trade[];
}
```

<a id="finalytics-runreport"></a>

### RunReport

One run of a strategy, summarised: its statistics, trading and the series its charts draw.

```typescript
export interface RunReport {
    /** The date the first period ends on. */
    start: Date;
    /** The date the last period ends on. */
    end: Date;
    /** Return, risk and ratios, against the risk-free return over the run. */
    statistics: Statistics;
    /** Historical value at risk and expected shortfall over one period, at 95%. */
    tail_risk: TailRisk;
    /** The annual risk-free return over the run. */
    risk_free_rate: number;
    /** Against the benchmark over the same periods, if there is one. */
    relative?: RelativeStatistics | null;
    /** The number of trades after the start. */
    rebalances: number;
    /** The share of value bought (or sold) in a year. */
    turnover_per_year: number;
    /** The cost of trading as a share of value in a year. */
    costs_per_year: number;
    /** The largest trade, as a share of value bought. */
    largest_trade: number;
    /**
     * The growth of 100, from 100 on the date the run enters (when it is known), at the end of
     * each month for daily or weekly returns, and of each period otherwise.
     */
    growth: Series;
    /**
     * The fall from the highest value so far, from 0 on the date the run enters: the deepest
     * in each month for daily or weekly returns, and at each period otherwise.
     */
    drawdown: Series;
    /**
     * The Sharpe ratio over the past three years, at the growth's dates once three years have
     * passed, when the run is longer than three years.
     */
    rolling_sharpe?: Series | null;
    /** The return in each calendar year. */
    years: CalendarReturn[];
    /** The return in each month, a row per year and a column per month. */
    months: Grid;
    /** The run followed with money, when the backtest follows it. */
    money?: MoneyFigures | null;
}
```

<a id="finalytics-samplepath"></a>

### SamplePath

One simulation's value at each recorded month.

```typescript
export interface SamplePath {
    /** The value at the start of each of the recorded months, in today's money. */
    values: number[];
    /** Whether this simulation meets the goal. */
    met: boolean;
}
```

<a id="finalytics-scenariodefinition"></a>

### ScenarioDefinition

What a scenario does: shocks, or a historical period replayed.

```typescript
export type ScenarioDefinition =
| {
    type: "shocks";
```

<a id="finalytics-scenariooptions"></a>

### ScenarioOptions

What to report beyond the effect on each holding.

```typescript
export interface ScenarioOptions {
    /**
     * Classification schemes to add the effect up by, such as `asset_class`. Instruments
     * without the classification are in the group "unclassified", and cash in "cash".
     */
    group_by?: string[];
    /** The risk model for the risk before and after; without one, no risk is reported. */
    risk?: ScenarioRiskModel | null;
}
```

<a id="finalytics-scenariooutcome"></a>

### ScenarioOutcome

One scenario's result, or why it could not be run.

```typescript
export interface ScenarioOutcome {
    /** The scenario's name. */
    name: string;
    /** The result; absent when the scenario could not be run. */
    result?: ScenarioResult | null;
    /** The problems that stopped it, with paths into the scenario. */
    problems: Issue[];
}
```

<a id="finalytics-scenariopath"></a>

### ScenarioPath

The hypothetical value of today's holdings over a historical period.

```typescript
export interface ScenarioPath {
    /** Every date in the period with a price for a holding, and its start and end. */
    dates: Date[];
    /** The holdings' value on each date, in the base currency. */
    values: number[];
    /**
     * The largest fall from a peak over the period, absent if the value never falls or is not
     * always above zero.
     */
    worst_drawdown?: Drawdown | null;
}
```

<a id="finalytics-scenarioresult"></a>

### ScenarioResult

What a scenario does to a portfolio.

```typescript
export interface ScenarioResult {
    /** The date of the holdings. */
    as_of: Date;
    /** The currency values are in. */
    base_currency: Currency;
    /** The portfolio's value before. */
    value_before: number;
    /** The portfolio's value after. */
    value_after: number;
    /** `value_after - value_before`. */
    change: number;
    /** `change / value_before`. */
    portfolio_return: number;
    /** Each holding, in holding order: instruments by identifier, then cash by currency. */
    holdings: HoldingImpact[];
    /** The instruments held whose return in their own currency no shock set. */
    not_shocked: InstrumentId[];
    /**
     * The factor returns applied, given and propagated, in the order of the model's factors;
     * empty without factor shocks.
     */
    factor_returns: Record<string, number>;
    /** The effect by each classification scheme the options group by. */
    groups: SchemeImpact[];
    /** The risk before, with a risk model. */
    risk_before?: ScenarioRisk | null;
    /**
     * The risk after, with the stressed covariance if the risk model has a stress; absent
     * without a risk model or if the portfolio is worth nothing after.
     */
    risk_after?: ScenarioRisk | null;
    /** For a historical scenario, the holdings' value over the period. */
    path?: ScenarioPath | null;
    /** Prices and rates carried forward from earlier dates, and shocks that matched nothing. */
    warnings: ValidationReport;
}
```

<a id="finalytics-scenariorisk"></a>

### ScenarioRisk

A portfolio's risk before or after a scenario.

```typescript
export interface ScenarioRisk {
    /**
     * The volatility of the portfolio's return, in the units of the risk model (scale the
     * model to annualise).
     */
    volatility: number;
    /**
     * The parametric value at risk of the portfolio's value at the model's confidence, a loss
     * in the base currency: `z * volatility * value`.
     */
    value_at_risk_base: number;
    /** Each holding's contribution to the volatility, in the order of the holdings. */
    contributions: RiskContribution[];
    /** The contributions added up by each classification scheme the options group by. */
    groups: SchemeRisk[];
}
```

<a id="finalytics-scenarioriskmodel"></a>

### ScenarioRiskModel

The risk model for the risk before and after a scenario.

```typescript
export interface ScenarioRiskModel {
    /**
     * The covariance of the holdings' returns, labelled with instrument identifiers and, for
     * cash, the currency code followed by " cash" (as holding returns are). Give this or
     * `factor_model`.
     */
    covariance?: Matrix | null;
    /** A factor model of the holdings' returns, labelled in the same way. */
    factor_model?: FactorModel | null;
    /** The confidence of the value at risk. The default is 0.95. */
    confidence?: number;
    /** A stress of the covariance for the risk after the scenario, as in a crisis. */
    stress?: CorrelationStress | null;
}
```

<a id="finalytics-scenariosummary"></a>

### ScenarioSummary

One scenario in a line, for a summary table or a bar chart.

```typescript
export interface ScenarioSummary {
    /** The scenario's name. */
    name: string;
    /** The change in value, in the base currency. */
    change: number;
    /** The portfolio's return. */
    portfolio_return: number;
    /** The annualised volatility after. */
    volatility_after?: number | null;
    /** The value at risk after, in the base currency, at the first confidence of the options. */
    value_at_risk_after_base?: number | null;
    /** The holding that lost most. */
    worst?: HoldingKey | null;
    /** Its change in value. */
    worst_change?: number | null;
    /** The stress of the covariance behind the risk after, if the scenario has one. */
    stress?: CorrelationStress | null;
}
```

<a id="finalytics-scenariosreport"></a>

### ScenariosReport

The scenarios report.

```typescript
export interface ScenariosReport {
    /** Where the numbers come from. */
    provenance: Provenance;
    /** The portfolio's value before every scenario. */
    value: number;
    /** The first date of the returns behind the risk model, when there is one. */
    risk_start?: Date | null;
    /** Each scenario, in the order given. */
    scenarios: ScenarioOutcome[];
    /** The scenarios that ran, in a line each. */
    summary: ScenarioSummary[];
    /** The change in each holding's value (rows) in each scenario that ran (columns). */
    changes: Grid;
    /** Each holding's return in the base currency in each scenario that ran. */
    returns: Grid;
}
```

<a id="finalytics-schemecontributions"></a>

### SchemeContributions

Contributions added up by the groups of one classification scheme.

```typescript
export interface SchemeContributions {
    /** The classification scheme. */
    scheme: string;
    /** Each group, in name order. */
    groups: GroupContribution[];
}
```

<a id="finalytics-schemeimpact"></a>

### SchemeImpact

The effect of a scenario by the groups of one classification scheme.

```typescript
export interface SchemeImpact {
    /** The classification scheme. */
    scheme: string;
    /** Each group, in name order. */
    groups: GroupImpact[];
}
```

<a id="finalytics-schemeperformance"></a>

### SchemePerformance

Performance by the groups of one classification scheme of the accounts.

```typescript
export interface SchemePerformance {
    /** The classification scheme. */
    scheme: string;
    /** Each group, in name order. */
    groups: AccountsPerformance[];
}
```

<a id="finalytics-schemerisk"></a>

### SchemeRisk

Risk by the groups of one classification scheme.

```typescript
export interface SchemeRisk {
    /** The classification scheme. */
    scheme: string;
    /** Each group's share of the volatility, in name order. */
    groups: GroupRisk[];
}
```

<a id="finalytics-screen"></a>

### Screen

A screen: the screening report and the raw results. Printing it prints the report, and
showing it shows the report as a page.

```typescript
export interface Screen {
    /**
     * The screening report: how the rules narrow the universe, the matches in order, the
     * securities that fail just one rule, where each threshold falls and every measure.
     */
    report: ScreenReport;
    /**
     * The raw results: one row per security, with each measure's value, its fields and
     * whether it matched.
     */
    results: ScreenResults;
}
```

<a id="finalytics-screencolumn"></a>

### ScreenColumn

A column of the matches: a measure or a field.

```typescript
export interface ScreenColumn {
    /** The measure's or the field's name, such as `"return_1y"` or `"sector"`. */
    name: string;
    /** Its name in words, such as "1-year return" or "Sector". */
    label: string;
    /** How a measure is shown; absent for a field. */
    unit?: MeasureUnit | null;
    /** Whether it is a field rather than a measure. */
    field: boolean;
}
```

<a id="finalytics-screencontext"></a>

### ScreenContext

What a screen measured, beside its figures: what the report needs to describe it.

```typescript
export interface ScreenContext {
    /** The engine's version. */
    version: string;
    /**
     * A SHA-256 digest, in hexadecimal, of the options and of what the screen found for each
     * security: the same inputs give the same report.
     */
    digest: string;
    /** The date measured to. */
    as_of: Date;
    /** The securities in the universe. */
    securities: number;
    /** The earliest close of any security. */
    first_date?: Date | null;
    /** The benchmark, if any. */
    benchmark?: string | null;
    /** The annual risk-free rate. */
    risk_free_rate: number;
    /** The most days a last close could be before the date measured to without being stale. */
    max_carry_days: number;
    /** The measures computed. */
    measures: Measure[];
    /** The names of the fields given, then of the formulas. */
    fields: string[];
    /** The formulas, in order, with how many securities have no value of each. */
    formulas?: FormulaResult[];
    /** How the matches are sorted. */
    sort?: Sort | null;
    /** The most matches kept. */
    limit?: number | null;
    /** Problems that did not stop the screen. */
    warnings: ValidationReport;
}
```

<a id="finalytics-screenexceptions"></a>

### ScreenExceptions

What is out of the ordinary in the securities' prices.

```typescript
export interface ScreenExceptions {
    /**
     * The securities measured on a last close before the date measured to, within
     * `max_carry_days` of it.
     */
    late: string[];
    /** The stale securities: listed, but not measured. */
    stale: string[];
    /** The securities with too short a history for a rule or the sort. */
    too_short: string[];
    /** The date of every security's first close, when they all share one. */
    common_start?: Date | null;
}
```

<a id="finalytics-screenmatch"></a>

### ScreenMatch

One security of the matches, with its value in each of the report's columns.

```typescript
export interface ScreenMatch {
    /** Its place in the sort, from 1. */
    rank: number;
    /** The security. */
    id: string;
    /** Its value in each column, in the order of the report's `columns`. */
    values: (FieldValue | null)[];
    /**
     * The places in `columns` of its values close to a threshold of a rule: within 10% of it
     * or, for a threshold of zero, within 10% of the measure's typical size, the median of its
     * size across the securities that are not stale.
     */
    near_threshold?: number[];
}
```

<a id="finalytics-screenoptions"></a>

### ScreenOptions

What a screen measures and keeps. Everything is optional: by default every measure the
universe allows, on its latest date, with no rules.

```typescript
export interface ScreenOptions {
    /** The date measured to: the latest date in the universe by default. */
    as_of?: Date | null;
    /**
     * The most days a security's last close may be before `as_of`; a security whose last close
     * is older is stale: listed, but not measured. 5 by default.
     */
    max_carry_days?: number;
    /**
     * The measures to compute, by name, such as `["return_1y", "rsi_14"]`: by default every
     * measure the universe allows (those against a benchmark need one, and those of volumes
     * and dividends need them). The measures the rules and the sort use are computed too.
     */
    measures?: Measure[] | null;
    /** The rules a security must all meet to match. */
    rules?: Rule[];
    /** How the matches are sorted; in the universe's order by default. */
    sort?: Sort | null;
    /** The most matches to keep, after sorting. */
    limit?: number | null;
    /**
     * Formulas by name, in order, such as `{"risk_adjusted": "return_1y / volatility"}`: each
     * a numeric field of every security, which rules, the sort and the results take as they
     * take a field. A formula is numbers, the names of measures, numeric fields and earlier
     * formulas, `+ - * /`, unary minus, parentheses, and `abs(x)`, `sqrt(x)`, `log(x)`
     * (natural), `min(a, b, ...)` and `max(a, b, ...)`. A security has no value where an input
     * has none, where the formula divides by zero, takes the square root of a negative number
     * or the logarithm of one at most zero, or where the result is not finite.
     */
    formulas?: Record<string, string>;
    /**
     * Whether the universe's benchmark is screened as a candidate too. By default it serves
     * the measures against a benchmark, such as beta, and is not one of the securities listed.
     */
    include_benchmark?: boolean;
}
```

<a id="finalytics-screenreport"></a>

### ScreenReport

The screening report: what one screen of a universe found.

```typescript
export interface ScreenReport {
    /** What was measured and how. */
    context: ScreenContext;
    /** How many securities were measured, passed and matched. */
    coverage: Coverage;
    /** Each rule, with how many met it. */
    rules: RuleResult[];
    /**
     * The columns of the matches: the security's name and sector when given, the text fields
     * the rules test, the measure or field sorted by and those the rules compare with numbers;
     * without rules or a sort, a few of the main measures.
     */
    columns: ScreenColumn[];
    /** Every match, in order. */
    matches: ScreenMatch[];
    /** The first securities that pass every rule but are beyond the limit, in order: up to ten. */
    also_passed?: ScreenMatch[];
    /**
     * Every security that fails just one rule, the closest to passing first: by how far its
     * value is from the rule's threshold, as a share of the threshold (or, for a threshold of
     * zero, of the measure's typical size). Those that fail a rule on text, or have no value
     * for it, come last.
     */
    near_misses?: NearMiss[];
    /**
     * For each measure or numeric field the rules compare with a number, every security's
     * value against the rules' thresholds.
     */
    strips?: ThresholdStrip[];
    /** The spread of each measure across the universe. */
    spread: MeasureSpread[];
    /** What is out of the ordinary in the securities' prices. */
    exceptions?: ScreenExceptions;
    /**
     * The report's title, such as "Quality at a fair price": shown when it is printed or laid
     * out as a page.
     */
    name?: string | null;
}
```

<a id="finalytics-screenresults"></a>

### ScreenResults

The raw results of a screen: one row per security, in the universe's order, with each
measure's value, its fields and whether it matched; the matches in order; each rule with how
many met it; and how many were measured.

```typescript
export interface ScreenResults {
    /** The date measured to. */
    as_of: Date;
    /** The most days a last close could be before `as_of` without being stale. */
    max_carry_days: number;
    /** The benchmark, if any. */
    benchmark?: string | null;
    /** The annual risk-free rate of the Sharpe and Sortino ratios. */
    risk_free_rate: number;
    /** The measures computed, in the order of each security's `values`. */
    measures: Measure[];
    /**
     * The names of the fields given, then of the formulas, which each security's `fields`
     * give as numbers.
     */
    fields: string[];
    /** The formulas, in order, with how many securities have no value of each. */
    formulas?: FormulaResult[];
    /** Each rule, with how many met it. */
    rules: RuleResult[];
    /** How the matches are sorted. */
    sort?: Sort | null;
    /** The most matches kept. */
    limit?: number | null;
    /** The matches kept, in order. */
    matches: string[];
    /** How many securities were measured, passed and matched, and how many lacked each measure. */
    coverage: Coverage;
    /** One row per security, in the universe's order. */
    securities: SecurityScreen[];
    /** Problems that did not stop the screen, such as closes left out. */
    warnings: ValidationReport;
}
```

<a id="finalytics-screenstatus"></a>

### ScreenStatus

Whether a security was measured.

```typescript
export type ScreenStatus = "measured" | "too_short" | "stale";
```

<a id="finalytics-securityscreen"></a>

### SecurityScreen

One security's row of a screen: its measures, its fields and whether it matched.

```typescript
export interface SecurityScreen {
    /** The security. */
    id: string;
    /** Whether it was measured. */
    status: ScreenStatus;
    /** Whether it meets every rule. */
    passed: boolean;
    /** Whether it is one of the matches kept: it passed, and is within the limit. */
    matched: boolean;
    /** Its place among the matches kept, from 1. */
    rank?: number | null;
    /** The date of its first close. */
    first_date?: Date | null;
    /** The date of its last close on or before the date measured to. */
    last_date?: Date | null;
    /**
     * Its value of each measure of the results' `measures`, in their order; absent where it
     * has none.
     */
    values: (number | null)[];
    /** Its fields, by name. */
    fields?: Record<string, FieldValue>;
}
```

<a id="finalytics-securityseries"></a>

### SecuritySeries

One security's prices on its own dates: a close on each date, and its volume, dividends and
fields if known.

```typescript
export interface SecuritySeries {
    /** The security's identifier, such as `"AAPL"`. */
    id: string;
    /** The dates of its closes, in increasing order. */
    dates: Date[];
    /**
     * Its close on each date. Adjusted closes, which include dividends and splits, give total
     * returns. A close that is not a number (NaN, or `null` in JSON) is no close: that date is
     * left out.
     */
    close: (number | null)[];
    /** The volume traded on each date, if known; NaN (`null`) where not. */
    volume?: (number | null)[] | null;
    /**
     * The dividend per share paid on each date, its ex-date, and zero on the others, if known;
     * NaN (`null`) where not known, which counts as none paid unless every one is unknown.
     */
    dividends?: (number | null)[] | null;
    /**
     * Values the library does not compute, by name, such as `{"sector": "Technology",
     * "pe_ratio": 18.4}`: numbers or text, for rules and results.
     */
    fields?: Record<string, FieldValue>;
}
```

<a id="finalytics-segment"></a>

### Segment

A segment of a portfolio and of its benchmark over one period: an asset class, a sector, a
region or any other grouping.

```typescript
export interface Segment {
    /** The segment's name. */
    name: string;
    /** The portfolio's weight in the segment at the start of the period. */
    portfolio_weight: number;
    /** The return of the portfolio's holdings in the segment over the period. */
    portfolio_return: number;
    /** The benchmark's weight in the segment at the start of the period. */
    benchmark_weight: number;
    /** The return of the benchmark's holdings in the segment over the period. */
    benchmark_return: number;
}
```

<a id="finalytics-segmenteffects"></a>

### SegmentEffects

What one segment added to the excess return.

```typescript
export interface SegmentEffects {
    /** The segment's name. */
    name: string;
    /**
     * The effect of weighting the segment differently from the benchmark,
     * `(w - b)(s - B)`.
     */
    allocation: number;
    /**
     * The effect of the portfolio's holdings in the segment returning differently from the
     * benchmark's, `b (r - s)`.
     */
    selection: number;
    /** The joint effect of the two, `(w - b)(r - s)`. */
    interaction: number;
    /** The sum of the three. */
    total: number;
}
```

<a id="finalytics-series"></a>

### Series

A labelled series of numbers over time, for a line or bar chart.

```typescript
export interface Series {
    /** What the series is, such as "Portfolio" or "Rolling volatility (63 days)". */
    name: string;
    /** What its values measure. */
    unit: Unit;
    /** The dates. */
    dates: Date[];
    /** One value per date. */
    values: number[];
}
```

<a id="finalytics-settings"></a>

### Settings

How a backtest is run.

```typescript
export interface Settings {
    /** How the strategies are validated: in sample by default. */
    validation?: Validation;
    /** When the portfolio trades back to its targets: quarterly by default. */
    rebalance?: RebalanceSchedule;
    /** The cost of trading, in basis points of the value traded: 10 is 0.1%. Zero by default. */
    trading_cost_bps?: number;
}
```

<a id="finalytics-severity"></a>

### Severity

How serious a validation issue is.

```typescript
export type Severity = "warning" | "error";
```

<a id="finalytics-shocksource"></a>

### ShockSource

How a holding's return in a scenario was set.

```typescript
export type ShockSource =
| {
    type: "instrument";
```

<a id="finalytics-shocks"></a>

### Shocks

The shocks of a scenario. An instrument's return in its own currency comes from its own
shock, or else from the one group shock that matches it, or else from the factor shocks; an
instrument none of them reaches returns zero and is listed as not shocked. A currency shock
compounds with the instrument's return, and applies to cash in that currency.

```typescript
export interface Shocks {
    /** Returns of single instruments, by identifier. */
    instruments?: Record<string, number>;
    /**
     * Returns of every instrument in a group. An instrument without its own shock may match at
     * most one of them.
     */
    groups?: GroupShock[];
    /** Changes in the value of currencies in the base currency: -0.1 when a currency loses 10%. */
    currencies?: Record<string, number>;
    /** Returns of factors. */
    factors?: FactorShocks | null;
}
```

<a id="finalytics-shortfalldecomposition"></a>

### ShortfallDecomposition

A portfolio's expected shortfall split into each asset's contribution (historical Euler
allocation).

```typescript
export interface ShortfallDecomposition {
    /** The confidence, for example 0.95. */
    confidence: number;
    /**
     * The portfolio's expected shortfall over one period of the scenarios, as a positive loss:
     * the mean loss in the worst `1 - confidence` of them (the Rockafellar–Uryasev tail mean,
     * which minimum CVaR minimises).
     */
    expected_shortfall: number;
    /**
     * Each asset's contribution: its weight times its mean loss in those scenarios, in the
     * order of the scenarios' columns. The components add up to the expected shortfall.
     */
    contributions: RiskContribution[];
}
```

<a id="finalytics-shortfallfigure"></a>

### ShortfallFigure

An expected shortfall over one period of the returns.

```typescript
export interface ShortfallFigure {
    /** The confidence, such as 0.95. */
    confidence: number;
    /** The mean loss in the worst `1 - confidence` of the periods, as a positive return. */
    value: number;
}
```

<a id="finalytics-side"></a>

### Side

Buy or sell.

```typescript
export type Side = "buy" | "sell";
```

<a id="finalytics-slice"></a>

### Slice

One part of a whole, for a pie, bar or treemap chart.

```typescript
export interface Slice {
    /** The part's name: a holding, a currency, a group. */
    name: string;
    /** Its value in the base currency. */
    value: number;
    /** Its share of the whole. */
    weight: number;
}
```

<a id="finalytics-solution"></a>

### Solution

The result of an optimisation, with what is needed to judge it: the status, the objective,
how closely the constraints hold, and the work done.

Two solutions are equal when everything but the time taken is equal, so that solving the
same problem twice gives equal solutions.

```typescript
export interface Solution {
    /** The objective optimised. */
    objective: Objective;
    /** How the optimisation ended. */
    status: SolveStatus;
    /** The asset labels. */
    labels: string[];
    /** The optimal weights, one per asset, when the status is optimal or nearly optimal. */
    weights?: number[] | null;
    /**
     * The value of the objective at the weights: the variance for minimum variance and target
     * return; the utility for mean–variance; the expected return for target volatility; the
     * Sharpe ratio; the diversification ratio; the CVaR, as a positive loss; the semivariance;
     * the CDaR, as a positive drawdown of the summed returns; for risk budgeting, the largest
     * difference between an asset's share of risk and its budget. Absent for hierarchical risk
     * parity, which optimises no objective. The downside measures are per period of the
     * scenarios.
     */
    objective_value?: number | null;
    /** The expected return at the weights, when expected returns are given. */
    expected_return?: number | null;
    /** The volatility at the weights, when a covariance matrix or a factor model is given. */
    volatility?: number | null;
    /**
     * The largest amount by which the weights break a constraint (0 when every constraint
     * holds exactly).
     */
    max_violation?: number | null;
    /** The method used. */
    method: SolveMethod;
    /** Iterations of the solver or of Newton's method. */
    iterations: number;
    /** The time taken, in seconds. It varies from run to run and is left out of comparisons. */
    solve_time: number;
}
```

<a id="finalytics-solvemethod"></a>

### SolveMethod

The method that produced a solution.

```typescript
export type SolveMethod = "interior_point" | "newton" | "recursive_bisection" | "closed_form" | "trade_elimination";
```

<a id="finalytics-solvestatus"></a>

### SolveStatus

How an optimisation ended.

```typescript
export type SolveStatus = "optimal" | "nearly_optimal" | "infeasible" | "unbounded" | "iteration_limit" | "numerical_difficulty";
```

<a id="finalytics-sort"></a>

### Sort

How matches are sorted: by a measure or a numeric field, one way or the other. Securities
without a value come last.

```typescript
export interface Sort {
    /** The measure sorted by; or give `field`. */
    measure?: Measure | null;
    /** The numeric field sorted by; or give `measure`. */
    field?: string | null;
    /** The largest first (`descending`) or the smallest first (`ascending`). */
    order: Order;
}
```

<a id="finalytics-source"></a>

### Source

What a report was computed from: a digest of the portfolio document and market data, and
their size. Computing it once and passing it to each report avoids hashing the inputs again.

```typescript
export interface Source {
    /**
     * The SHA-256 digest, in hexadecimal, of the portfolio document with its market data, as
     * the engine writes it. The same inputs give the same digest in every language.
     */
    digest: string;
    /** The number of transactions. */
    transactions: number;
    /** The number of accounts. */
    accounts: number;
    /** The number of instruments. */
    instruments: number;
}
```

<a id="finalytics-statistics"></a>

### Statistics

Statistics of a returns series.

Annualised figures scale by the number of periods in a year given in the options. Ratios
that divide by a dispersion are absent when the returns do not vary.

```typescript
export interface Statistics {
    /** The number of returns. */
    observations: number;
    /** The compounded return over the whole series. */
    total_return: number;
    /** The compound annual growth rate: `(1 + total_return)^(periods_per_year / observations) - 1`. */
    annualised_return: number;
    /** The annualised standard deviation of the returns (sample, divisor n - 1). */
    volatility: number;
    /**
     * The annualised root mean square of the shortfalls below the target return, over all
     * observations.
     */
    downside_deviation: number;
    /**
     * The mean return in excess of the risk-free rate over its standard deviation, annualised by
     * the square root of `periods_per_year`.
     */
    sharpe_ratio?: number | null;
    /** The annualised mean return in excess of the target over the downside deviation. */
    sortino_ratio?: number | null;
    /**
     * The largest fall from a peak, as a negative return (-0.25 for a fall of 25%), measured
     * from a starting value before the first return.
     */
    max_drawdown: number;
    /**
     * The conditional drawdown at risk at 95%: the mean of the worst 5% of the drawdowns after
     * each return, as a negative return like `max_drawdown` (Chekhlov, Uryasev and Zabarankin,
     * 2005). It is the Rockafellar–Uryasev tail mean, as the expected shortfall of minimum CVaR
     * is, of the drawdowns measured on the compounded value.
     */
    conditional_drawdown_at_risk: number;
    /**
     * The Ulcer index: the root mean square of the drawdowns after each return, as a positive
     * fraction (Martin, 1987). It weighs how deep and how long the falls are.
     */
    ulcer_index: number;
    /** The annualised return over the size of the maximum drawdown; absent without a drawdown. */
    calmar_ratio?: number | null;
    /** The skewness of the returns (population moments). */
    skewness?: number | null;
    /**
     * The kurtosis of the returns less 3, so that a normal distribution has 0 (population
     * moments).
     */
    excess_kurtosis?: number | null;
    /** The best single return. */
    best: number;
    /** The worst single return. */
    worst: number;
}
```

<a id="finalytics-statisticsoptions"></a>

### StatisticsOptions

Conventions for `statistics` and `relative_statistics`.

```typescript
export interface StatisticsOptions {
    /**
     * Return periods in a year: 252 for daily trading-day returns (the default), 52 for weekly,
     * 12 for monthly.
     */
    periods_per_year?: number;
    /**
     * The annual risk-free rate for the Sharpe ratio and alpha. It is converted to a rate per
     * period by compounding: `(1 + rate)^(1 / periods_per_year) - 1`. The default is 0.
     */
    risk_free_rate?: number;
    /**
     * The annual minimum acceptable return for downside deviation and the Sortino ratio,
     * converted to a rate per period in the same way. The default is 0.
     */
    target_return?: number;
}
```

<a id="finalytics-status"></a>

### Status

Where a plan stands against the chance wanted.

```typescript
export type Status = "room_to_spare" | "on_track" | "nearly_on_track" | "off_track";
```

<a id="finalytics-strategy"></a>

### Strategy

A strategy to backtest.

```typescript
export type Strategy =
| {
    type: "optimise";
```

<a id="finalytics-strategyreport"></a>

### StrategyReport

One strategy, scored on the periods it was tested on, with its weights over time.

```typescript
export interface StrategyReport {
    /** The strategy's name. */
    name: string;
    /**
     * Whether a method fitted the weights to returns: false for fixed weights and for the
     * benchmark.
     */
    fitted: boolean;
    /**
     * The run the strategy is scored on: out of sample for a holdout or walk-forward
     * validation, in sample otherwise.
     */
    tested: RunReport;
    /**
     * For a holdout or walk-forward validation, the run in sample: on the history before the
     * test for a holdout, and with weights fitted to the whole history over the same periods
     * as the test for walk-forward. Fixed weights have none, as they were not fitted; the
     * benchmark has the same periods as the strategies' run in sample, for reference.
     */
    in_sample?: RunReport | null;
    /** The weights bought at the start of the test, then at each refit. */
    weights: WeightsAt[];
    /** Each refit that found no weights, when the weights held were kept. */
    failures: Failure[];
    /** Anything the reader should know about how the strategy was run. */
    notes: string[];
}
```

<a id="finalytics-strategyresult"></a>

### StrategyResult

One strategy's backtest.

`in_sample` holds the returns of weights fitted to the history they are tested on: the
whole history for in-sample validation; the history before the test for a holdout; and, for
walk-forward validation, the whole history, tested on the same periods as `out_of_sample`, so
that the two can be compared directly. `out_of_sample` holds the returns of weights tested on
history they had not seen; it is absent for in-sample validation.

```typescript
export interface StrategyResult {
    /** The strategy's name. */
    name: string;
    /**
     * Whether a method fitted the weights to returns: false for fixed weights and for the
     * benchmark, which were not fitted to any history.
     */
    fitted: boolean;
    /**
     * The returns of weights tested on the history they were fitted to; for fixed weights,
     * the same periods as a fitted strategy's.
     */
    in_sample: Run;
    /** The returns of weights tested on history they had not seen. */
    out_of_sample?: Run | null;
    /** Each refit that found no weights, when the weights held were kept. */
    failures: Failure[];
    /**
     * Anything the reader should know about how the strategy was run, such as limits that did
     * not apply to it.
     */
    notes: string[];
}
```

<a id="finalytics-strippoint"></a>

### StripPoint

One security's value on a threshold strip.

```typescript
export interface StripPoint {
    /** The security. */
    id: string;
    /** Its value. */
    value: number;
    /** Whether it is one of the matches. */
    matched: boolean;
}
```

<a id="finalytics-tailmethod"></a>

### TailMethod

How the tail of the return distribution is estimated.

```typescript
export type TailMethod = "historical" | "parametric" | "cornish_fisher";
```

<a id="finalytics-tailrisk"></a>

### TailRisk

Value at risk and expected shortfall over one period of the returns.

Both are losses, reported as positive returns: a value at risk of 0.021 at 95% confidence
means that one period in twenty is expected to lose 2.1% or more.

```typescript
export interface TailRisk {
    /** The confidence level, for example 0.95. */
    confidence: number;
    /** The estimation method. */
    method: TailMethod;
    /** The loss exceeded with probability `1 - confidence`. */
    value_at_risk: number;
    /**
     * The expected loss when the value at risk is exceeded (also called conditional value at
     * risk).
     */
    expected_shortfall: number;
    /**
     * Whether the expected shortfall was raised to the value at risk. The Cornish–Fisher
     * expansion can put the mean of the tail above its quantile when the excess kurtosis is
     * large; the expected shortfall is never below the value at risk, so it is set to it.
     */
    shortfall_at_var: boolean;
}
```

<a id="finalytics-tailrow"></a>

### TailRow

Value at risk and expected shortfall by one method at one confidence.

```typescript
export interface TailRow {
    /** The confidence. */
    confidence: number;
    /** The method. */
    method: TailMethod;
    /** The value at risk over one valuation period, a loss. */
    value_at_risk: Amount;
    /** The expected shortfall over one valuation period, a loss. */
    expected_shortfall: Amount;
    /**
     * Whether the expected shortfall was raised to the value at risk, as the Cornish–Fisher
     * expansion can need when the excess kurtosis is large.
     */
    shortfall_at_var: boolean;
}
```

<a id="finalytics-taxrates"></a>

### TaxRates

Tax rates on realised gains, for tax-aware sales. A lot held for more than
`long_term_after_days` days is long-term. The rates apply to gains in the base currency,
after fees; a loss saves tax at its rate.

```typescript
export interface TaxRates {
    /** The rate on gains from lots held for at most `long_term_after_days` days. */
    short_term: Decimal;
    /** The rate on gains from lots held for longer. */
    long_term: Decimal;
    /** The holding period, in days, beyond which a gain is long-term: 365 by default. */
    long_term_after_days?: number;
}
```

<a id="finalytics-thresholdstrip"></a>

### ThresholdStrip

Where the securities' values of one measure, or of a numeric field, fall against the
thresholds of the rules that test it.

```typescript
export interface ThresholdStrip {
    /** The measure's or the field's name. */
    name: string;
    /** Its name in words. */
    label: string;
    /** How a measure is shown; absent for a field. */
    unit?: MeasureUnit | null;
    /** The places in the report's `rules` of the rules that test it. */
    rules: number[];
    /** The value of every security that is not stale and has one, lowest first. */
    points: StripPoint[];
}
```

<a id="finalytics-timeseries"></a>

### TimeSeries

Values indexed by strictly increasing dates.

Construction checks the ordering, so every `TimeSeries` can be searched by date. A series is
serialised as `{"dates": [...], "values": [...]}`, and deserialising checks it again.

```typescript
export interface TimeSeries {
    dates: Date[];
    values: Decimal[];
}
```

<a id="finalytics-trackingerrorlimit"></a>

### TrackingErrorLimit

A limit on ex-ante tracking error against a benchmark.

```typescript
export interface TrackingErrorLimit {
    /** The benchmark's weights, one per asset. */
    benchmark: number[];
    /**
     * The largest tracking error allowed, `sqrt((w - b)' C (w - b))`, in the units of the
     * covariance matrix's volatility.
     */
    limit: number;
}
```

<a id="finalytics-trade"></a>

### Trade

A trade back to target weights, or to new ones after a refit.

```typescript
export interface Trade {
    /** The date at whose end the trade is made. */
    date: Date;
    /** The weights after the trade, by asset; what they leave is held in cash. */
    weights: Record<string, number>;
    /**
     * The share of the portfolio's value bought (or, equally, sold): half the sum of the
     * changes in weight, cash included.
     */
    turnover: number;
    /** The cost of the trade as a share of the portfolio's value. */
    cost: number;
    /** Whether the weights were refitted, rather than restored. */
    refit: boolean;
}
```

<a id="finalytics-tradelimits"></a>

### TradeLimits

Limits on the trades from the current portfolio (the turnover's current weights). They are
not convex, so the solver's portfolio is adjusted by a heuristic, reported as the method
`trade_elimination`, which is not guaranteed to find the best portfolio within the limits.

```typescript
export interface TradeLimits {
    /**
     * The smallest trade allowed, as a weight: each asset is either not traded or traded by at
     * least this much.
     */
    minimum?: number | null;
    /** The largest number of assets traded. */
    maximum_count?: number | null;
}
```

<a id="finalytics-traderow"></a>

### TradeRow

One proposed trade.

```typescript
export interface TradeRow {
    /** Buy or sell. */
    side: Side;
    /** The units traded. */
    quantity: number;
    /** The instrument's identifier. */
    instrument: string;
    /** The price per unit, in the instrument's currency. */
    price: number;
    /** The instrument's currency. */
    currency: Currency;
    /** The value traded, in the base currency. */
    value_base: number;
    /** The fee, in the instrument's currency. */
    fees_local: number;
    /** For a sale, the gain it realises, after fees. */
    realised_gain_base?: number | null;
    /** For a sale in a taxable account with tax rates, the estimated tax. */
    tax_base?: number | null;
}
```

<a id="finalytics-tradessummary"></a>

### TradesSummary

The trades from the weights held now to the weights found.

```typescript
export interface TradesSummary {
    /** The one-way turnover: half the sum of the absolute changes in weight, cash included. */
    turnover: number;
    /** The number of assets traded, cash aside. */
    count: number;
    /** Each asset traded, and cash, the largest change first. */
    trades: WeightChange[];
}
```

<a id="finalytics-transaction"></a>

### Transaction

One event in the life of a portfolio.

Transactions are applied in trade-date order; transactions on the same date are applied in the
order given. Positions are booked on the trade date; the settlement date is recorded for cash
reporting.

```typescript
export type Transaction = {
    /** The caller's identifier, unique within the portfolio. */
    id: TransactionId;
```

<a id="finalytics-transactionid"></a>

### TransactionId

Identifies a transaction. Lots opened by a transaction take their identity from it.

Identifiers are non-empty, have no leading or trailing whitespace, and are compared
exactly (case matters).

```typescript
export type TransactionId = string;
```

<a id="finalytics-turnover"></a>

### Turnover

Trading from the current portfolio: a limit on turnover, costs of trading, or both.

```typescript
export interface Turnover {
    /** The current weights, one per asset. */
    current: number[];
    /**
     * The largest one-way turnover, half the absolute weight changes, `sum |w - current| / 2`:
     * 0.2 allows 20% of the portfolio to be bought (and, equally, sold).
     */
    limit?: number | null;
    /**
     * Costs per unit of weight traded, one per asset, in the units of the expected returns.
     * They reduce the expected return of objectives that use one.
     */
    costs?: number[] | null;
    /**
     * Costs that grow with the square of the weight traded, one per asset, such as the market
     * impact of large trades: `sum k_i (w_i - current_i)^2`, in the units of the expected
     * returns. They reduce the expected return of mean–variance, robust mean–variance and
     * target volatility objectives.
     */
    quadratic_costs?: number[] | null;
}
```

<a id="finalytics-unit"></a>

### Unit

What the numbers of a series, grid or column measure.

```typescript
export type Unit =
| {
    type: "money";
```

<a id="finalytics-validation"></a>

### Validation

How a strategy's weights are tested: on the history they were fitted to, or on later
history they had not seen.

```typescript
export type Validation =
| {
    type: "in_sample";
```

<a id="finalytics-validationreport"></a>

### ValidationReport

Every problem found while validating an input, rather than only the first.

Each issue carries a path that locates it in the input, so that a user interface can point
to the exact field.

```typescript
export interface ValidationReport {
    issues: Issue[];
}
```

<a id="finalytics-valuation"></a>

### Valuation

The holdings valued on a date.

```typescript
export interface Valuation {
    /** The valuation date. */
    date: Date;
    /** The base currency. */
    base_currency: Currency;
    /** The positions, by account and instrument. */
    positions: PositionValue[];
    /** The cash balances, by account and currency. */
    cash: CashValue[];
    /** Positions and cash, in the base currency. */
    total_base: Decimal;
    /** Prices and rates carried forward from earlier dates, and cash balances below zero. */
    warnings: ValidationReport;
}
```

<a id="finalytics-valueseries"></a>

### ValueSeries

Daily values and external flows in the base currency, the input to performance measurement.

```typescript
export interface ValueSeries {
    /** The dates. */
    dates: Date[];
    /** The value of the portfolio at the end of each date. */
    values: Decimal[];
    /**
     * The external flows since the previous date (deposits and transfers in positive;
     * withdrawals and transfers out negative), each converted at its own date's rate.
     */
    flows: Decimal[];
    /** Prices and rates carried forward from earlier dates. */
    warnings: ValidationReport;
}
```

<a id="finalytics-view"></a>

### View

A view: the expected return of a portfolio of assets.

```typescript
export interface View {
    /**
     * The view portfolio: each asset's weight in it, such as `{"A": 1}` for an absolute view on
     * asset A, or `{"A": 1, "B": -1}` for a view that A outperforms B.
     */
    portfolio: Record<string, number>;
    /** The expected return of the view portfolio, in the units of the covariance matrix. */
    expected_return: number;
    /** How uncertain the view is. */
    uncertainty?: ViewUncertainty;
}
```

<a id="finalytics-viewuncertainty"></a>

### ViewUncertainty

How uncertain a view is: its variance in the model.

```typescript
export type ViewUncertainty =
| {
    type: "proportional";
```

<a id="finalytics-weightchange"></a>

### WeightChange

One asset's weight now and in the weights found.

```typescript
export interface WeightChange {
    /** The asset, or the risk-free asset for cash. */
    asset: string;
    /** Its weight now. */
    current: number;
    /** Its weight in the weights found. */
    proposed: number;
    /** `proposed - current`: bought when positive, sold when negative. */
    change: number;
    /** Whether this is the cash: the risk-free asset, or what the weights leave. */
    cash?: boolean;
}
```

<a id="finalytics-weightsat"></a>

### WeightsAt

Weights at a date, by asset; what they leave is held in cash.

```typescript
export interface WeightsAt {
    /** The date at whose end the weights were bought. */
    date: Date;
    /** The weights, by asset. */
    weights: Record<string, number>;
}
```

<a id="finalytics-whatif"></a>

### WhatIf

The plan when markets do worse than the simulations assume.

```typescript
export interface WhatIf {
    /** Every simulated month's return a percentage point a year lower. */
    lower_returns: Outcome;
    /**
     * The worst twelve months of the history placed before the target's date or at the first
     * payment, in every simulation; absent with less than twelve months to place them in.
     */
    crash?: Outcome | null;
    /** The months placed. */
    crash_window?: CrashWindow | null;
}
```

<a id="finalytics-yearreturn"></a>

### YearReturn

The return of one calendar year (or the part of it covered).

```typescript
export interface YearReturn {
    /** The year. */
    year: number;
    /**
     * The valuation the year's return starts from: the last one of the year before, or the
     * first valuation.
     */
    start: Date;
    /** The last valuation of the year. */
    end: Date;
    /** Whether the valuations cover the whole year. */
    complete: boolean;
    /** The portfolio's time-weighted return. */
    portfolio: number;
    /** The benchmark's return. */
    benchmark?: number | null;
    /** `portfolio - benchmark`. */
    excess?: number | null;
}
```

<a id="finalytics-assets"></a>

### Assets

The prices or returns of several assets over the same periods, with the settings that turn
them into annual figures, for statistics and optimisation.

```typescript
export class Assets {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The backtest of one strategy: `options.method`, fitted as the validation says, or fixed
     * `options.weights` by asset (adding up to one with the cash, as every input of weights may
     * hold it), against the
     * benchmark if one is given: the backtest report in `report`, and every return and trade
     * in `results`.
     */
    backtest(options: BacktestChoices): Result<BacktestReport, BacktestResults>;
    /**
     * The comparison of `options.methods`, each backtested on these assets with the same
     * limits, rebalancing, validation and trading costs: the comparison report in `report`,
     * and every return and trade in `results`.
     */
    compare(options: CompareChoices): Result<ComparisonReport, BacktestResults>;
    /**
     * The correlation of the assets' returns.
     */
    correlation(): Matrix;
    /**
     * The covariance of the assets' returns, annualised.
     */
    covariance(): CovarianceEstimate;
    /**
     * `options.weights` by asset valued on the estimates optimisation uses: the figures in
     * `results`, with the optimisation report's page and text for them, titled "Given
     * weights", in `report`. The limits apply to the efficient frontier, drawn with
     * `frontier: true`.
     */
    evaluate(options: EvaluateChoices): Result<OptimisationReport, Evaluation>;
    /**
     * Assets from prices in CSV text: a `date` column and one column per asset.
     */
    static fromCsv(text: string, options?: AssetsOptions): Assets;
    /**
     * Assets from their returns: the date each return ends on, the labels, and one column of
     * returns per asset.
     */
    static fromReturns(dates: string[], labels: string[], columns: (Float64Array | number[])[], options?: AssetsOptions): Assets;
    /**
     * The efficient frontier: `points` portfolios (20 by default) of the risky assets, under
     * the limits of `options`. It refuses a method and the methods' parameters, which it does
     * not use.
     */
    frontier(points: number | null | undefined, options?: FrontierOptions): FrontierPortfolio[];
    /**
     * Each asset's mean return, annualised, by the `mean_returns` estimator (the risk-free
     * asset's is its historical mean): the expected returns optimisation uses unless others are
     * given.
     */
    meanReturns(): Record<string, number>;
    /**
     * Assets from prices: dates, labels and one column of prices per asset (`null` where there
     * is none). Dates on which an asset has no price are left out, with a warning.
     */
    constructor(prices: PriceTable, options?: AssetsOptions);
    /**
     * The optimisation of the risky assets by the method of `options` (the largest Sharpe
     * ratio by default), fully invested in them unless `cash` limits let it hold cash or
     * `keep_cash` keeps the cash held, within its limits: the optimisation report in
     * `report`, and the weights, in the order of the assets, with the solver's solution in
     * `results`.
     */
    optimise(options?: OptimiseChoices): Result<OptimisationReport, OptimisationResult>;
    /**
     * The chance that a plan of saving, spending and investing meets `options.goal`, from
     * `options.simulations` markets drawn from these assets' monthly returns: the planning
     * report in `report`, and the raw results in `results`.
     */
    plan(options: PlanChoices | PlanOptions): Result<PlanReport, PlanResults>;
    /**
     * Each asset's statistics, with its value at risk and expected shortfall over one period
     * at 95%, in the order of the assets.
     */
    statistics(): AssetStatistics[];
    /**
     * The cash among the assets: the risk-free asset, if any, then cash in other currencies,
     * such as a portfolio's euro cash. Optimisation holds the risk-free asset only with `cash`
     * limits, keeps cash at its current weight with `keep_cash`, and otherwise invests it.
     */
    readonly cashAssets: string[];
    /**
     * Each classification scheme's class of each asset it classifies.
     */
    readonly classes: Record<string, Record<string, string>>;
    /**
     * The date each return ends on.
     */
    readonly dates: string[];
    /**
     * The assets.
     */
    readonly labels: string[];
    /**
     * The number of return periods in a year.
     */
    readonly periodsPerYear: number;
    /**
     * The returns, one array per asset.
     */
    readonly returns: Float64Array[];
    /**
     * The risk-free asset, if one of the assets is.
     */
    readonly riskFreeAsset: string | undefined;
    /**
     * The annual risk-free rate.
     */
    readonly riskFreeRate: number;
    /**
     * The assets optimisation chooses among by default: all but the cash. With `cash` limits,
     * it holds the risk-free asset, or cash at the risk-free rate, beside them.
     */
    readonly riskyAssets: string[];
    /**
     * Problems met reading the prices that did not stop the assets being built.
     */
    readonly warnings: Issue[];
}
```

<a id="finalytics-marketdata"></a>

### MarketData

Prices and FX rates for valuing a portfolio. Each method that adds data returns a new
`MarketData`.

Prices are in each instrument's currency; an FX rate is the number of units of `quote` per
unit of `base`. A rate can be found directly, inverted, or crossed through the pivot currency
(USD by default). A missing value may be carried forward from an earlier date for at most
`maxCarryDays` days (5 by default).

```typescript
export class MarketData {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Market data from CSV text of prices and FX rates, in long or wide layout.
     */
    static fromCsv(prices?: string | null, fx?: string | null, delimiter?: string | null): MarketData;
    /**
     * Market data in the portfolio document's format: `{prices, fx, pivot, max_carry_days}`.
     */
    static fromDocument(value: DocumentMarketData): MarketData;
    /**
     * Units of `quote` per unit of `base` on `date`, or `undefined` if no rate is found.
     */
    fxRate(base: string, quote: string, date: string): Observed | undefined;
    /**
     * Empty market data.
     */
    constructor();
    /**
     * The price of `id` on `date`, or `undefined` if there is none within the carry limit.
     */
    price(id: string, date: string): Observed | undefined;
    /**
     * These market data in the portfolio document's format.
     */
    toDocument(): DocumentMarketData;
    /**
     * These market data with rates for `base` priced in `quote`.
     */
    withFx(base: string, quote: string, dates: string[], rates: (Decimal | number)[]): MarketData;
    /**
     * These market data carrying values forward for at most `days` days.
     */
    withMaxCarryDays(days: number): MarketData;
    /**
     * These market data crossing rates through `pivot`.
     */
    withPivot(pivot: string): MarketData;
    /**
     * These market data with prices for `id` on `dates`.
     */
    withPrices(id: string, dates: string[], values: (Decimal | number)[]): MarketData;
    /**
     * How many days a price or rate may be carried forward.
     */
    readonly maxCarryDays: number;
    /**
     * The currency through which rates are crossed.
     */
    readonly pivot: string;
    /**
     * Every date with a price for any instrument, in order.
     */
    readonly priceDates: string[];
}
```

<a id="finalytics-performance"></a>

### Performance

A portfolio's daily time-weighted returns and the measures built on them.

```typescript
export class Performance {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Each holding's contribution to the time-weighted return; with `expenses`, they add up to
     * the return.
     */
    contributions(start?: string | null, end?: string | null): Contribution[];
    /**
     * The return over a period against a benchmark's, given as levels on dates.
     */
    excessReturn(benchmarkDates: string[], benchmarkLevels: Float64Array | number[], which?: Period | string, asOf?: Date, maxCarryDays?: number): ExcessReturn | undefined;
    /**
     * What fees and taxes charged to the accounts rather than to a holding took from the
     * time-weighted return: negative, or zero without them.
     */
    expenses(start?: string | null, end?: string | null): number;
    /**
     * Performance from end-of-day values and the external flows since the previous date
     * (positive into the portfolio; none by default).
     */
    static fromValues(dates: string[], values: Float64Array | number[], flows: Float64Array | number[] | null | undefined, timing?: FlowTiming): Performance;
    /**
     * The Modified Dietz return.
     */
    modifiedDietz(start?: string | null, end?: string | null): number | undefined;
    /**
     * The money-weighted return (internal rate of return, annualised).
     */
    moneyWeighted(start?: string | null, end?: string | null): number | undefined;
    /**
     * The return over a reporting period ending on `asOf`, annualised for a year or longer.
     */
    period(which: Period | "MTD" | "QTD" | "YTD" | "ITD" | `${number}Y`, asOf?: string | null): PeriodReturn | undefined;
    /**
     * The time-weighted return from `start` to `end` (by default the whole history).
     */
    timeWeighted(start?: string | null, end?: string | null): number | undefined;
    /**
     * The valuation dates.
     */
    readonly dates: string[];
    /**
     * The growth of 1 invested on the first date, one value per date.
     */
    readonly growth: Float64Array;
    /**
     * The daily returns, one per date after the first.
     */
    readonly returns: Float64Array;
    /**
     * The portfolio's value on each date, in the base currency.
     */
    readonly values: Float64Array;
    /**
     * Prices and rates carried forward while valuing the portfolio.
     */
    readonly warnings: Issue[];
}
```

<a id="finalytics-portfolio"></a>

### Portfolio

A portfolio: its records (base currency, accounts, instruments and transactions), the market
data that values it, and the books kept from them. It does not change: `withTransactions` and
`withRebalance` return new portfolios.

```typescript
export class Portfolio {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Each account's end-of-day value and external flows on `dates` (by default
     * `dates()`).
     */
    accountSeries(dates?: string[] | null): AccountSeries;
    /**
     * The holdings at the last price date, as `Assets`: their returns in the base currency on
     * the portfolio's dates, for statistics and optimisation. The instruments are the assets
     * optimisation chooses among (`riskyAssets`); the cash is there too (`cashAssets`), so that
     * the portfolio's own `weights()` can be valued and optimised from on the same estimates:
     * base-currency cash, such as `USD cash`, as the risk-free asset, riskless and earning
     * nothing, and cash in other currencies, such as `EUR cash`, moving with its exchange rate.
     * The returns start on the first date every instrument has a price, as the overview
     * report's estimates do.
     */
    assets(): Assets;
    /**
     * Brinson–Fachler attribution against the benchmark segments in the options.
     */
    attributionReport(options: Partial<ReportOptions>): AttributionReport;
    /**
     * The dates the portfolio is measured on when none are given: every date with a price,
     * from the first transaction to the last price.
     */
    dates(): string[];
    /**
     * What each transaction did to the books, in the order applied.
     */
    events(): Event[];
    /**
     * A portfolio from the CSV text of its files; only the transactions are required. Every
     * problem in every file is reported at once, by file, line and column.
     */
    static fromCsv(files: PortfolioFiles): Portfolio;
    /**
     * A portfolio from a portfolio document in JSON text.
     */
    static fromJson(text: string): Portfolio;
    /**
     * What the price and exchange-rate changes over a historical period would do to the
     * holdings, with their value on each date of the period.
     */
    historicalScenario(window: HistoricalWindow, choices?: ScenarioChoices): ScenarioResult;
    /**
     * Price returns of `holdings` in the base currency between consecutive `dates`.
     */
    holdingReturns(holdings: HoldingKey[], dates?: string[] | null): HoldingReturns;
    /**
     * Each holding's daily value, trade cash and income on `dates`.
     */
    holdingSeries(dates?: string[] | null): HoldingSeries;
    /**
     * Positions, lots and cash by holding and by account; allocation by every dimension;
     * concentration; realised gains and income.
     */
    holdingsReport(options?: Partial<ReportOptions>): HoldingsReport;
    /**
     * Positions, lots and cash at the end of `date`, or after the last transaction.
     */
    holdings(date?: string | null): Holdings;
    /**
     * Performance over `dates` (by default `dates()`): every account together, each
     * account, and the groups of accounts by each classification scheme in the options, with
     * each one's contribution to the household's time-weighted return.
     */
    household(dates?: Date[], options?: HouseholdOptions): Household;
    /**
     * The lots a sale would close, chosen by `method` (first in, first out by default).
     */
    lotsForSale(date: string, account: string, id: string, quantity: Decimal | number, method?: LotMethod): LotSale[];
    /**
     * A portfolio from its records and market data (pass `new MarketData()` for none): the
     * books are kept by applying the transactions in order.
     */
    constructor(data: PortfolioData, market: MarketData);
    /**
     * Headline figures, growth, allocation, risk by holding and by group, each holding's
     * expected return and risk, and the efficient frontier.
     */
    overviewReport(options?: Partial<ReportOptions>): OverviewReport;
    /**
     * Value and growth, returns over the standard periods, monthly and calendar-year returns,
     * contribution, flows and income, and the household.
     */
    performanceReport(options?: Partial<ReportOptions>): PerformanceReport;
    /**
     * Time-weighted performance on `dates` (by default `dates()`), with flows at the
     * start (default) or end of the day.
     */
    performance(dates?: Date[], timing?: FlowTiming): Performance;
    /**
     * Every realised gain, by lot.
     */
    realisedGains(): RealisedGain[];
    /**
     * The trades that move the accounts (by default every account; `choices.accounts` names
     * others) together to `targets`, as shares of their combined value: weights by holding, or
     * an optimisation of the portfolio's assets, whose weights are its targets (one that found
     * no weights is a `ValidationError`). The rebalance report is in `report`, and each
     * account's proposal in `results`; `withRebalance` makes the trades.
     */
    rebalance(targets: Record<HoldingKey, number> | Result<OptimisationReport, OptimisationResult> | OptimisationResult, choices?: RebalanceChoices): Result<RebalanceReport, RebalanceResults>;
    /**
     * Every report on the portfolio, from one set of options. The attribution report needs
     * `attribution` and the scenarios report `scenarios`; without them, they are absent.
     * `name`, such as `"Morgan family portfolio"`, is shown under the report's title.
     */
    report(options?: ReportChoices): PortfolioReport & ReportMethods;
    /**
     * Statistics, drawdowns, tail risk, the return distribution, the holdings' covariance,
     * correlation and risk decomposition, and rolling statistics.
     */
    riskReport(options?: Partial<ReportOptions>): RiskReport;
    /**
     * The scenarios in the options, run together on the current holdings, side by side.
     */
    scenariosReport(options: Partial<ReportOptions>): ScenariosReport;
    /**
     * What `shocks` would do to the holdings, with their risk before and after if the choices
     * give a risk model.
     */
    shockScenario(shocks: Shocks, choices?: ScenarioChoices): ScenarioResult;
    /**
     * The digest and size of the inputs the reports are computed from.
     */
    source(): Source;
    /**
     * The portfolio document of the portfolio and its market data, in JSON text.
     */
    toJson(): string;
    /**
     * The holdings valued at the end of `date`, by default the last price date.
     */
    valuation(date?: string | null): Valuation;
    /**
     * End-of-day values and external flows on `dates` (by default `dates()`).
     */
    valueSeries(dates?: string[] | null): ValueSeries;
    /**
     * Each holding's share of the portfolio's value at the end of `date`, by default the last
     * price date.
     */
    weights(date?: string | null): Record<HoldingKey, number>;
    /**
     * The same portfolio with `goals` kept with it in place of its own: each a plan with a
     * name, no two the same.
     */
    withGoals(goals: PlanOptions[]): Portfolio;
    /**
     * The same portfolio valued with other market data.
     */
    withMarket(market: MarketData): Portfolio;
    /**
     * The same portfolio with the trades of a rebalancing made on its date: the result of
     * `rebalance`, or its raw results.
     */
    withRebalance(proposal: Result<RebalanceReport, RebalanceResults> | RebalanceResults): Portfolio;
    /**
     * The same portfolio with more transactions, applied after its own.
     */
    withTransactions(transactions: Transaction[]): Portfolio;
    /**
     * The last date with a price: the date the portfolio is valued on when no other is given.
     */
    readonly asOf: string | undefined;
    /**
     * The currency results are reported in.
     */
    readonly baseCurrency: string;
    /**
     * The records: base currency, accounts, instruments and transactions.
     */
    readonly data: PortfolioData;
    /**
     * The goals kept with the portfolio, each a plan with its name, for `Assets.plan`.
     */
    readonly goals: PlanOptions[];
    /**
     * The market data.
     */
    readonly market: MarketData;
}
```

<a id="finalytics-universe"></a>

### Universe

Many securities, each with its own dates and closes, and their volumes, dividends and fields
where known, with the benchmark and risk-free rate a screen measures them against.

```typescript
export class Universe {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * A universe from flat columns, for large universes: every security's rows one after
     * another, `starts` giving the row each security starts at, `dates` as days since 1970-01-01
     * in an `Int32Array` (or ISO strings), and `close`, `volume` and `dividends` as
     * `Float64Array`s, NaN where there is no value. `fields` gives each field's values in the
     * order of `ids`: a `Float64Array` or an array of numbers, or an array of text. The options
     * give the benchmark and the risk-free rate.
     */
    static fromColumns(columns: UniverseColumns, options?: UniverseSettings): Universe;
    /**
     * A universe from prices in CSV text: the long layout of `id`, `date` and `close` columns,
     * with `volume` and `dividends` if known, or a `date` column and one column per security.
     * The options give the fields, as CSV text or by security, the benchmark and the risk-free
     * rate.
     */
    static fromCsv(text: string, options?: UniverseOptions): Universe;
    /**
     * A universe from series, one per security, each on its own dates: `{ id, dates, close }`,
     * with `volume`, `dividends` and `fields` if known. Dates are ISO strings or days since
     * 1970-01-01 in an `Int32Array`; closes, volumes and dividends are `Float64Array`s or arrays
     * of numbers. A close that is not a number is no close: its date is left out. The options
     * give the benchmark, the risk-free rate and more fields, as CSV text or by security.
     */
    constructor(series: SeriesInput[], options?: UniverseOptions);
    /**
     * The screen of `options`: every security measured on its own history up to `as_of`,
     * judged on the rules, with the matches sorted and limited: the screening report in
     * `report`, and in `results` one array per column, in the order of `ids`, with one
     * `Float64Array` per measure (NaN where a security has no value).
     */
    screen(options?: ScreenChoices): Result<ScreenReport, ScreenColumns>;
    /**
     * The benchmark, if one is set.
     */
    readonly benchmark: string | undefined;
    /**
     * The names of the fields given.
     */
    readonly fields: string[];
    /**
     * The securities, in order.
     */
    readonly ids: string[];
    /**
     * The latest date of any security's closes: the date a screen measures to by default.
     */
    readonly latestDate: string | undefined;
    /**
     * The number of securities.
     */
    readonly length: number;
    /**
     * The annual risk-free rate.
     */
    readonly riskFreeRate: number;
    /**
     * Problems met reading the securities that did not stop the universe being built.
     */
    readonly warnings: Issue[];
}
```

<a id="finalytics-blacklitterman"></a>

### blackLitterman

The Black–Litterman posterior expected returns and covariance.

```typescript
export function blackLitterman(model: BlackLitterman): BlackLittermanEstimate;
```

<a id="finalytics-brinsonfachler"></a>

### brinsonFachler

Brinson–Fachler attribution by segment over one or more periods, linked by `linking`
(`"carino"` by default, or `"menchero"`).

```typescript
export function brinsonFachler(periods: AttributionPeriod[], linking?: Linking): Attribution;
```

<a id="finalytics-correlation"></a>

### correlation

The correlation matrix of a covariance matrix.

```typescript
export function correlation(matrix: Matrix): Matrix;
```

<a id="finalytics-covariance"></a>

### covariance

Estimates the covariance matrix of a table of returns (one column per asset). The method is
`{method: "sample"}` by default; others are `"ewma"` (with `decay`), `"ledoit_wolf"` and
`"oas"`.

```typescript
export function covariance(returns: ReturnsTable, method?: CovarianceMethod): CovarianceEstimate;
```

<a id="finalytics-datasets"></a>

### datasets

Datasets for examples, tests and practice: a two-account household holding five real funds,
synthetic asset-class indices and a synthetic universe of 200 companies for screening. The
funds' prices are real, from Yahoo Finance; the household's transactions are illustrative.

```typescript
export class datasets {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Month-end levels, from October 1996 to October 2026, of six asset-class indices that
     * start at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate.
     */
    static assetClassPrices(): PriceTable;
    /**
     * The text of a dataset file, such as `"portfolio/transactions.csv"`: an example of the
     * layout each file takes.
     */
    static file(name: string): string;
    /**
     * The names of the dataset files, for `datasets.file`.
     */
    static files(): string[];
    /**
     * A portfolio of two accounts in US dollars and euros, from November 2023 to October 2026:
     * five real funds at their real prices, with illustrative transactions and a balanced
     * benchmark, `BENCH`.
     */
    static portfolio(): Portfolio;
    /**
     * The options of the full report on `datasets.portfolio()`.
     */
    static reportOptions(): ReportOptions;
    /**
     * A universe of 200 fictional companies in eleven sectors, for screening: their daily
     * closes on every weekday from November 2020 to October 2026, with volumes and quarterly
     * dividends, and the market index `INDEX` as the benchmark, at a risk-free rate of 4% a
     * year. Each company has the fields `name`, `sector`, `market_cap` (in US dollars),
     * `trailing_pe` (none for the companies that make a loss) and `price_to_book`, and is
     * identified by the first four letters of its name. The data is synthetic, drawn with a
     * fixed seed; every platform draws the same universe.
     */
    static universe(): Universe;
}
```

<a id="finalytics-drawdownseries"></a>

### drawdownSeries

The fall below the running peak on each date, as a negative return.

```typescript
export function drawdownSeries(dates: string[], values: Float64Array | number[]): Float64Array;
```

<a id="finalytics-drawdowns"></a>

### drawdowns

Every drawdown in a series of levels, in date order.

```typescript
export function drawdowns(dates: string[], values: Float64Array | number[]): Drawdown[];
```

<a id="finalytics-factormodel"></a>

### factorModel

Checks a factor model built from exposures and risk estimates supplied from elsewhere,
returning it unchanged.

```typescript
export function factorModel(model: FactorModel): FactorModel;
```

<a id="finalytics-factorrisk"></a>

### factorRisk

Splits a portfolio's volatility under a factor model into each factor's contribution and the
specific part, and into each asset's contribution.

```typescript
export function factorRisk(weights: Float64Array | number[], model: FactorModel): FactorRisk;
```

<a id="finalytics-factortrackingerror"></a>

### factorTrackingError

The ex-ante tracking error against benchmark weights under a factor model, split by factor
and by asset.

```typescript
export function factorTrackingError(weights: Float64Array | number[], benchmark: Float64Array | number[], model: FactorModel): FactorRisk;
```

<a id="finalytics-frontier"></a>

### frontier

The efficient frontier: `points` portfolios (20 by default) from the minimum-variance
portfolio to the largest attainable expected return.

```typescript
export function frontier(problem: Problem, points?: number | null): FrontierPoint[];
```

<a id="finalytics-fundamentalmodel"></a>

### fundamentalModel

Estimates a fundamental factor model for given exposures, one row per asset and one entry per
factor: each period's returns are regressed on the exposures to estimate the factor returns.

```typescript
export function fundamentalModel(returns: ReturnsTable, factors: string[], exposures: number[][]): FactorModelEstimate;
```

<a id="finalytics-impliedcovariance"></a>

### impliedCovariance

The covariance matrix of the assets that a factor model implies, `B F B' + diag(d)`.

```typescript
export function impliedCovariance(model: FactorModel): Matrix;
```

<a id="finalytics-impliedriskaversion"></a>

### impliedRiskAversion

The market's risk aversion implied by its returns: the annualised excess return over the
annualised variance. Options default to 252 periods a year and a zero risk-free rate.

```typescript
export function impliedRiskAversion(returns: Float64Array | number[], options?: { periods_per_year?: number; risk_free_rate?: number }): number;
```

<a id="finalytics-karnoskysinger"></a>

### karnoskySinger

Currency attribution by the method of Karnosky and Singer over one or more periods.

```typescript
export function karnoskySinger(periods: CurrencyAttributionPeriod[], linking?: Linking): CurrencyAttribution;
```

<a id="finalytics-methods"></a>

### methods

Every optimisation method with what it takes: its parameters, with their defaults and the
values allowed, and the limits on weights it accepts. `Assets.optimise` checks its options
against the same descriptions.

```typescript
export function methods(): MethodInfo[];
```

<a id="finalytics-normalcdf"></a>

### normalCdf

The standard normal distribution function.

```typescript
export function normalCdf(x: number): number;
```

<a id="finalytics-normalpdf"></a>

### normalPdf

The standard normal density.

```typescript
export function normalPdf(x: number): number;
```

<a id="finalytics-normalquantile"></a>

### normalQuantile

The standard normal quantile.

```typescript
export function normalQuantile(p: number): number;
```

<a id="finalytics-readdocument"></a>

### readDocument

Reads a portfolio document, reporting where a malformed one goes wrong.

```typescript
export function readDocument(text: string): Document;
```

<a id="finalytics-rebalancetransactions"></a>

### rebalanceTransactions

A rebalancing proposal as transactions, ready to add to the portfolio.

```typescript
export function rebalanceTransactions(plan: Rebalance, prefix?: string | null): Transaction[];
```

<a id="finalytics-regressionmodel"></a>

### regressionModel

Estimates a factor model by regressing each asset's returns on factor returns, with an
intercept. Both tables cover the same periods; the model is per period.

```typescript
export function regressionModel(returns: ReturnsTable, factorReturns: ReturnsTable): FactorModelEstimate;
```

<a id="finalytics-relativestatistics"></a>

### relativeStatistics

Statistics of a returns series against a benchmark's over the same periods.

```typescript
export function relativeStatistics(returns: Float64Array | number[], benchmark: Float64Array | number[], options?: Partial<StatisticsOptions>): RelativeStatistics;
```

<a id="finalytics-reporthtml"></a>

### reportHtml

The six reports from `Portfolio.report`, or the report of `Assets.compare` or
`Assets.backtest`, as one self-contained HTML page, with `name`, if given, as its title:
inline styles and SVG charts, no scripts, in light and dark themes, ready to print.

```typescript
export function reportHtml(report: PortfolioReport | OptimisationReport | ComparisonReport | BacktestReport | RebalanceReport | PlanReport | ScreenReport, name: string | null | undefined, theme: Theme): string;
```

<a id="finalytics-reporttables"></a>

### reportTables

Any report, as returned by the engine, as labelled tables for export; the six reports of
`Portfolio.report` give the tables of each in turn.

```typescript
export function reportTables(report: object): ReportTable[];
```

<a id="finalytics-reporttext"></a>

### reportText

Any report, the six from `Portfolio.report`, or the report of `Assets.compare` or
`Assets.backtest`, as text for printing, 80 columns wide, with `name`, if given, under each
title.

```typescript
export function reportText(report: object, name?: string | null): string;
```

<a id="finalytics-resampled"></a>

### resampled

Resampled optimisation: the average of the optimal portfolios for estimates re-made from
simulated histories. The same seed gives the same result on every platform.

```typescript
export function resampled(problem: Problem, objective: Objective, resampling: Resampling): ResampledSolution;
```

<a id="finalytics-resultstext"></a>

### resultsText

The raw results of `Assets.optimise`, `Assets.compare`, `Assets.backtest` or `Assets.plan` as
text: the weights, each strategy's periods, growth, trades and refits, or a plan's chance.

```typescript
export function resultsText(results: OptimisationResult | BacktestResults | PlanResults | ScreenResults): string;
```

<a id="finalytics-riskdecomposition"></a>

### riskDecomposition

Splits a portfolio's volatility into each asset's marginal and component risk.

```typescript
export function riskDecomposition(weights: Float64Array | number[], matrix: Matrix): RiskDecomposition;
```

<a id="finalytics-rollingrelativestatistics"></a>

### rollingRelativeStatistics

`relativeStatistics` over every window of `window` returns.

```typescript
export function rollingRelativeStatistics(returns: Float64Array | number[], benchmark: Float64Array | number[], window: number, options?: Partial<StatisticsOptions>): RelativeStatistics[];
```

<a id="finalytics-rollingstatistics"></a>

### rollingStatistics

`statistics` over every window of `window` returns.

```typescript
export function rollingStatistics(returns: Float64Array | number[], window: number, options?: Partial<StatisticsOptions>): Statistics[];
```

<a id="finalytics-rollingtailrisk"></a>

### rollingTailRisk

`tailRisk` over every window of `window` returns.

```typescript
export function rollingTailRisk(returns: Float64Array | number[], window: number, confidence?: number, method?: TailMethod): TailRisk[];
```

<a id="finalytics-shortfalldecomposition"></a>

### shortfallDecomposition

Splits the expected shortfall at `confidence` (0.95 by default) of a portfolio with
`weights` of the assets of `scenarios` into each asset's contribution: its weight times its
mean loss in the worst scenarios. The contributions add up to the expected shortfall.

```typescript
export function shortfallDecomposition(weights: Float64Array | number[], scenarios: ReturnsTable, confidence?: number | null): ShortfallDecomposition;
```

<a id="finalytics-solve"></a>

### solve

The optimal portfolio for `objective`. A problem with no solution returns a solution whose
`status` says why and whose `weights` are null.

```typescript
export function solve(problem: Problem, objective: Objective): Solution;
```

<a id="finalytics-statisticalmodel"></a>

### statisticalModel

Estimates a statistical factor model from the first `factors` principal components of the
sample covariance matrix. The factors are named `pc1`, `pc2` and so on.

```typescript
export function statisticalModel(returns: ReturnsTable, factors: number): FactorModelEstimate;
```

<a id="finalytics-statistics"></a>

### statistics

Statistics of a returns series of at least two periods. Options default to 252 periods a
year and zero risk-free and target rates.

```typescript
export function statistics(returns: Float64Array | number[], options?: Partial<StatisticsOptions>): Statistics;
```

<a id="finalytics-stressedcovariance"></a>

### stressedCovariance

`covariance` with every volatility scaled and every correlation moved toward one by
`stress`: `s^2 ((1 - a) C + a v v')`, with `v` the volatilities.

```typescript
export function stressedCovariance(covariance: Matrix, stress: CorrelationStress): Matrix;
```

<a id="finalytics-tailrisk"></a>

### tailRisk

Value at risk and expected shortfall over one period, as positive losses.

```typescript
export function tailRisk(returns: Float64Array | number[], confidence?: number, method?: TailMethod): TailRisk;
```

<a id="finalytics-trackingerrordecomposition"></a>

### trackingErrorDecomposition

The ex-ante tracking error against `benchmark` weights, split into each asset's contribution.

```typescript
export function trackingErrorDecomposition(weights: Float64Array | number[], benchmark: Float64Array | number[], matrix: Matrix): RiskDecomposition;
```

<a id="finalytics-validateportfolio"></a>

### validatePortfolio

Every structural problem in a portfolio, without applying its transactions.

```typescript
export function validatePortfolio(portfolio: Portfolio): Issue[];
```

<a id="finalytics-version"></a>

### version

The version of this release of Finalytics.

```typescript
export function version(): string;
```

<a id="finalytics-writedocument"></a>

### writeDocument

A portfolio document holding `portfolio` and, optionally, its market data, as JSON text. The
market data is a `MarketData`, as `readDocument` returns it, or the document format
(`market.toDocument()`).

```typescript
export function writeDocument(portfolio: Portfolio, market?: MarketData | DocumentMarketData): string;
```

<a id="finalytics-initinput"></a>

### InitInput

No additional description in the package source.

```typescript
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
```

<a id="finalytics-initoutput"></a>

### InitOutput

No additional description in the package source.

```typescript
export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly assets_backtest: (a: number, b: any) => [number, number, number];
    readonly assets_cashAssets: (a: number) => [number, number];
    readonly assets_classes: (a: number) => [number, number, number];
    readonly assets_compare: (a: number, b: any) => [number, number, number];
    readonly assets_correlation: (a: number) => [number, number, number];
    readonly assets_covariance: (a: number) => [number, number, number];
    readonly assets_dates: (a: number) => [number, number];
    readonly assets_evaluate: (a: number, b: any) => [number, number, number];
    readonly assets_fromCsv: (a: number, b: number, c: any) => [number, number, number];
    readonly assets_fromReturns: (a: number, b: number, c: number, d: number, e: any, f: any) => [number, number, number];
    readonly assets_frontier: (a: number, b: number, c: any) => [number, number, number];
    readonly assets_labels: (a: number) => [number, number];
    readonly assets_meanReturns: (a: number) => [number, number, number];
    readonly assets_new: (a: any, b: any) => [number, number, number];
    readonly assets_optimise: (a: number, b: any) => [number, number, number];
    readonly assets_periodsPerYear: (a: number) => number;
    readonly assets_plan: (a: number, b: any) => [number, number, number];
    readonly assets_returns: (a: number) => any;
    readonly assets_riskFreeAsset: (a: number) => [number, number];
    readonly assets_riskFreeRate: (a: number) => number;
    readonly assets_riskyAssets: (a: number) => [number, number];
    readonly assets_statistics: (a: number) => [number, number, number];
    readonly assets_warnings: (a: number) => [number, number, number];
    readonly blackLitterman: (a: any) => [number, number, number];
    readonly brinsonFachler: (a: any, b: any) => [number, number, number];
    readonly correlation: (a: any) => [number, number, number];
    readonly covariance: (a: any, b: any) => [number, number, number];
    readonly datasets_assetClassPrices: () => [number, number, number];
    readonly datasets_file: (a: number, b: number) => [number, number, number, number];
    readonly datasets_files: () => [number, number];
    readonly datasets_portfolio: () => number;
    readonly datasets_reportOptions: () => [number, number, number];
    readonly datasets_universe: () => number;
    readonly drawdownSeries: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly drawdowns: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly factorModel: (a: any) => [number, number, number];
    readonly factorRisk: (a: number, b: number, c: any) => [number, number, number];
    readonly factorTrackingError: (a: number, b: number, c: number, d: number, e: any) => [number, number, number];
    readonly frontier: (a: any, b: number) => [number, number, number];
    readonly fundamentalModel: (a: any, b: any, c: any) => [number, number, number];
    readonly impliedCovariance: (a: any) => [number, number, number];
    readonly impliedRiskAversion: (a: number, b: number, c: any) => [number, number, number];
    readonly karnoskySinger: (a: any, b: any) => [number, number, number];
    readonly marketdata_fromCsv: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly marketdata_fromDocument: (a: any) => [number, number, number];
    readonly marketdata_fxRate: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly marketdata_maxCarryDays: (a: number) => number;
    readonly marketdata_new: () => number;
    readonly marketdata_pivot: (a: number) => [number, number];
    readonly marketdata_price: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly marketdata_priceDates: (a: number) => [number, number];
    readonly marketdata_toDocument: (a: number) => [number, number, number];
    readonly marketdata_withFx: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any) => [number, number, number];
    readonly marketdata_withMaxCarryDays: (a: number, b: number) => number;
    readonly marketdata_withPivot: (a: number, b: number, c: number) => [number, number, number];
    readonly marketdata_withPrices: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number, number];
    readonly methods: () => [number, number, number];
    readonly normalCdf: (a: number) => number;
    readonly normalPdf: (a: number) => number;
    readonly normalQuantile: (a: number) => number;
    readonly performance_contributions: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly performance_dates: (a: number) => [number, number];
    readonly performance_excessReturn: (a: number, b: number, c: number, d: number, e: number, f: any, g: any, h: any) => [number, number, number];
    readonly performance_expenses: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly performance_fromValues: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly performance_growth: (a: number) => [number, number];
    readonly performance_modifiedDietz: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly performance_moneyWeighted: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly performance_period: (a: number, b: any, c: number, d: number) => [number, number, number];
    readonly performance_returns: (a: number) => [number, number];
    readonly performance_timeWeighted: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly performance_values: (a: number) => [number, number];
    readonly performance_warnings: (a: number) => [number, number, number];
    readonly portfolio_accountSeries: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_asOf: (a: number) => [number, number];
    readonly portfolio_assets: (a: number) => [number, number, number];
    readonly portfolio_attributionReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_baseCurrency: (a: number) => [number, number];
    readonly portfolio_data: (a: number) => [number, number, number];
    readonly portfolio_dates: (a: number) => [number, number];
    readonly portfolio_events: (a: number) => [number, number, number];
    readonly portfolio_fromCsv: (a: any) => [number, number, number];
    readonly portfolio_fromJson: (a: number, b: number) => [number, number, number];
    readonly portfolio_goals: (a: number) => [number, number, number];
    readonly portfolio_historicalScenario: (a: number, b: any, c: any) => [number, number, number];
    readonly portfolio_holdingReturns: (a: number, b: any, c: number, d: number) => [number, number, number];
    readonly portfolio_holdingSeries: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_holdings: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_holdingsReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_household: (a: number, b: any, c: any) => [number, number, number];
    readonly portfolio_lotsForSale: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number, j: number) => [number, number, number];
    readonly portfolio_market: (a: number) => number;
    readonly portfolio_new: (a: any, b: number) => [number, number, number];
    readonly portfolio_overviewReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_performance: (a: number, b: any, c: number, d: number) => [number, number, number];
    readonly portfolio_performanceReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_realisedGains: (a: number) => [number, number, number];
    readonly portfolio_rebalance: (a: number, b: any, c: any) => [number, number, number];
    readonly portfolio_report: (a: number, b: any) => [number, number, number];
    readonly portfolio_riskReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_scenariosReport: (a: number, b: any) => [number, number, number];
    readonly portfolio_shockScenario: (a: number, b: any, c: any) => [number, number, number];
    readonly portfolio_source: (a: number) => [number, number, number];
    readonly portfolio_toJson: (a: number) => [number, number];
    readonly portfolio_valuation: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_valueSeries: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_weights: (a: number, b: number, c: number) => [number, number, number];
    readonly portfolio_withGoals: (a: number, b: any) => [number, number, number];
    readonly portfolio_withMarket: (a: number, b: number) => [number, number, number];
    readonly portfolio_withRebalance: (a: number, b: any) => [number, number, number];
    readonly portfolio_withTransactions: (a: number, b: any) => [number, number, number];
    readonly readDocument: (a: number, b: number) => [number, number, number];
    readonly rebalanceTransactions: (a: any, b: number, c: number) => [number, number, number];
    readonly regressionModel: (a: any, b: any) => [number, number, number];
    readonly relativeStatistics: (a: number, b: number, c: number, d: number, e: any) => [number, number, number];
    readonly reportHtml: (a: any, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly reportTables: (a: any) => [number, number, number];
    readonly reportText: (a: any, b: number, c: number) => [number, number, number, number];
    readonly resampled: (a: any, b: any, c: any) => [number, number, number];
    readonly resultsText: (a: any) => [number, number, number, number];
    readonly riskDecomposition: (a: number, b: number, c: any) => [number, number, number];
    readonly rollingRelativeStatistics: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number, number];
    readonly rollingStatistics: (a: number, b: number, c: number, d: any) => [number, number, number];
    readonly rollingTailRisk: (a: number, b: number, c: number, d: any, e: number, f: number) => [number, number, number];
    readonly shortfallDecomposition: (a: number, b: number, c: any, d: number, e: number) => [number, number, number];
    readonly solve: (a: any, b: any) => [number, number, number];
    readonly statisticalModel: (a: any, b: number) => [number, number, number];
    readonly statistics: (a: number, b: number, c: any) => [number, number, number];
    readonly stressedCovariance: (a: any, b: any) => [number, number, number];
    readonly tailRisk: (a: number, b: number, c: any, d: number, e: number) => [number, number, number];
    readonly trackingErrorDecomposition: (a: number, b: number, c: number, d: number, e: any) => [number, number, number];
    readonly universe_benchmark: (a: number) => [number, number];
    readonly universe_fields: (a: number) => [number, number];
    readonly universe_fromColumns: (a: any, b: any) => [number, number, number];
    readonly universe_fromCsv: (a: number, b: number, c: any) => [number, number, number];
    readonly universe_ids: (a: number) => [number, number];
    readonly universe_latestDate: (a: number) => [number, number];
    readonly universe_length: (a: number) => number;
    readonly universe_new: (a: any, b: any) => [number, number, number];
    readonly universe_riskFreeRate: (a: number) => number;
    readonly universe_screen: (a: number, b: any) => [number, number, number];
    readonly universe_warnings: (a: number) => [number, number, number];
    readonly validatePortfolio: (a: any) => [number, number, number];
    readonly version: () => [number, number];
    readonly writeDocument: (a: any, b: any) => [number, number, number, number];
}
```

<a id="finalytics-syncinitinput"></a>

### SyncInitInput

No additional description in the package source.

```typescript
export type SyncInitInput = BufferSource | WebAssembly.Module;
```

<a id="finalytics-initsync"></a>

### initSync

Instantiates the given `module`, which can either be bytes or
a precompiled `WebAssembly.Module`.

@param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.

@returns {InitOutput}

```typescript
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;
```

<a id="finalytics-init"></a>

### init

If `module_or_path` is {RequestInfo} or {URL}, makes a request and
for everything else, calls `WebAssembly.instantiate` directly.

@param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.

@returns {Promise<InitOutput>}

```typescript
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
```

## finalytics/client.js

<a id="finalytics-client-js-workerlike"></a>

### WorkerLike

A message port that a worker speaks through: a browser `Worker` or a Node `Worker`.

```typescript
export interface WorkerLike {
  postMessage(message: unknown): void;
  terminate(): unknown;
  addEventListener?(type: "message", listener: (event: { data: unknown }) => void): void;
  on?(type: "message", listener: (data: unknown) => void): void;
}
```

<a id="finalytics-client-js-remoteobject"></a>

### RemoteObject

An object held in the worker.

```typescript
export interface RemoteObject<T> {
  readonly handle: number;
  /** Calls a method of the object in the worker. */
  call<K extends keyof Methods<T>>(
    name: K,
    ...args: Parameters<Extract<T[K], (...args: never[]) => unknown>>
  ): Promise<Remote<ReturnType<Extract<T[K], (...args: never[]) => unknown>>>>;
  /** Reads a property of the object in the worker. */
  get<K extends keyof T>(name: K): Promise<T[K]>;
  /** Frees the object in the worker. */
  free(): Promise<void>;
}
```

<a id="finalytics-client-js-remoteportfolio"></a>

### RemotePortfolio

No additional description in the package source.

```typescript
export type RemotePortfolio = RemoteObject<finalytics.Portfolio>;
```

<a id="finalytics-client-js-remoteassets"></a>

### RemoteAssets

No additional description in the package source.

```typescript
export type RemoteAssets = RemoteObject<finalytics.Assets>;
```

<a id="finalytics-client-js-remoteuniverse"></a>

### RemoteUniverse

No additional description in the package source.

```typescript
export type RemoteUniverse = RemoteObject<finalytics.Universe>;
```

<a id="finalytics-client-js-remoteperformance"></a>

### RemotePerformance

No additional description in the package source.

```typescript
export type RemotePerformance = RemoteObject<finalytics.Performance>;
```

<a id="finalytics-client-js-remotemarketdata"></a>

### RemoteMarketData

No additional description in the package source.

```typescript
export type RemoteMarketData = RemoteObject<finalytics.MarketData>;
```

<a id="finalytics-client-js-engine"></a>

### Engine

The engine running in a worker.

```typescript
export interface Engine {
  /** Calls an exported function in the worker. */
  call<K extends keyof Functions>(name: K, ...args: Parameters<Functions[K]>): Promise<ReturnType<Functions[K]>>;
  /** A portfolio in the worker, from its records and market data in the document format. */
  portfolio(data: finalytics.PortfolioData, market?: finalytics.DocumentMarketData): Promise<RemotePortfolio>;
  /** A portfolio in the worker, from the CSV text of its files. */
  portfolioFromCsv(files: finalytics.PortfolioFiles): Promise<RemotePortfolio>;
  /** Assets in the worker, from their prices. */
  assets(prices: finalytics.PriceTable, options?: finalytics.AssetsOptions): Promise<RemoteAssets>;
  /** A universe in the worker, from series, one per security, each on its own dates. */
  universe(series: finalytics.SeriesInput[], options?: finalytics.UniverseOptions): Promise<RemoteUniverse>;
  /**
   * Calls a static method of `Portfolio`, `Assets`, `Universe`, `MarketData` or `datasets` in the
   * worker, such as `Assets.fromCsv`, `Universe.fromColumns` or `datasets.file`. Objects it returns
   * stay in the worker.
   */
  static<C extends keyof Classes, K extends keyof Methods<Classes[C]>>(
    owner: C,
    name: K,
    ...args: Parameters<Static<C, K>>
  ): Promise<Remote<ReturnType<Static<C, K>>>>;
  /** Stops the worker. */
  terminate(): unknown;
}
```

<a id="finalytics-client-js-createengine"></a>

### createEngine

An engine running in `worker`, which runs `finalytics/worker.js`.

```typescript
export function createEngine(worker: WorkerLike): Engine;
```
