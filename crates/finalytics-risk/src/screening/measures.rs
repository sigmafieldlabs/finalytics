//! The measures a screen computes, and how each is computed from one security's prices.

use serde::{Deserialize, Serialize};

/// Trading days in a year, by which daily figures are annualised.
pub(crate) const DAYS_A_YEAR: f64 = 252.0;

/// A measure a screen computes for each security, from its own prices up to the date measured
/// to. Each is named in options and results by its snake-case name, such as `"return_1y"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Measure {
    /// The return over the last week (7 days).
    #[serde(rename = "return_1w")]
    Return1w,
    /// The return over the last month.
    #[serde(rename = "return_1m")]
    Return1m,
    /// The return over the last three months.
    #[serde(rename = "return_3m")]
    Return3m,
    /// The return over the last six months.
    #[serde(rename = "return_6m")]
    Return6m,
    /// The return since the last day of the previous year.
    #[serde(rename = "return_ytd")]
    ReturnYtd,
    /// The return over the last year.
    #[serde(rename = "return_1y")]
    Return1y,
    /// The return over the last three years, a year: `(1 + r)^(1/3) - 1`.
    #[serde(rename = "return_3y")]
    Return3y,
    /// The return over the last five years, a year: `(1 + r)^(1/5) - 1`.
    #[serde(rename = "return_5y")]
    Return5y,
    /// The annualised standard deviation of the daily returns over the last year.
    #[serde(rename = "volatility")]
    Volatility,
    /// The annualised standard deviation of the daily returns over the last three years.
    #[serde(rename = "volatility_3y")]
    Volatility3y,
    /// The annualised root mean square of the daily returns' shortfalls below the risk-free
    /// rate over the last year.
    #[serde(rename = "downside_deviation")]
    DownsideDeviation,
    /// The largest fall from a peak over the last year, as a negative return.
    #[serde(rename = "max_drawdown")]
    MaxDrawdown,
    /// The largest fall from a peak over the last three years, as a negative return.
    #[serde(rename = "max_drawdown_3y")]
    MaxDrawdown3y,
    /// The fall from the highest close of the whole history to the last, as a negative return.
    #[serde(rename = "drawdown")]
    Drawdown,
    /// The mean daily return in excess of the risk-free rate over its standard deviation,
    /// annualised, over the last year.
    #[serde(rename = "sharpe_ratio")]
    SharpeRatio,
    /// The Sharpe ratio over the last three years.
    #[serde(rename = "sharpe_ratio_3y")]
    SharpeRatio3y,
    /// The annualised mean daily return in excess of the risk-free rate over the downside
    /// deviation, over the last year.
    #[serde(rename = "sortino_ratio")]
    SortinoRatio,
    /// The three-year return a year over the size of the three-year maximum drawdown.
    #[serde(rename = "calmar_ratio")]
    CalmarRatio,
    /// The loss of one day exceeded on one day in twenty, from the daily returns of the last
    /// year (historical, 95%), as a positive return.
    #[serde(rename = "value_at_risk")]
    ValueAtRisk,
    /// The covariance of the daily returns with the benchmark's over the benchmark's variance,
    /// over the last year.
    #[serde(rename = "beta")]
    Beta,
    /// The correlation of the daily returns with the benchmark's over the last year.
    #[serde(rename = "correlation")]
    Correlation,
    /// The return over the last six months less the benchmark's.
    #[serde(rename = "relative_return_6m")]
    RelativeReturn6m,
    /// The return over the last year less the benchmark's.
    #[serde(rename = "relative_return_1y")]
    RelativeReturn1y,
    /// The last close over the average of the last 20.
    #[serde(rename = "price_to_sma_20")]
    PriceToSma20,
    /// The last close over the average of the last 50.
    #[serde(rename = "price_to_sma_50")]
    PriceToSma50,
    /// The last close over the average of the last 200.
    #[serde(rename = "price_to_sma_200")]
    PriceToSma200,
    /// 1 when the average of the last 50 closes is above the average of the last 200, else 0.
    #[serde(rename = "sma_50_above_200")]
    Sma50Above200,
    /// The relative strength index of the last 14 changes, with Wilder's smoothing, from 0 to
    /// 100.
    #[serde(rename = "rsi_14")]
    Rsi14,
    /// The MACD line (the 12-day less the 26-day exponential average of the closes) less its
    /// 9-day exponential average, in the units of the price.
    #[serde(rename = "macd_histogram")]
    MacdHistogram,
    /// The last close over the highest close of the last year, less one: zero or negative.
    #[serde(rename = "from_52w_high")]
    From52wHigh,
    /// The last close over the lowest close of the last year, less one: zero or positive.
    #[serde(rename = "from_52w_low")]
    From52wLow,
    /// The annualised standard deviation of the last 20 daily returns.
    #[serde(rename = "volatility_20d")]
    Volatility20d,
    /// The dividends paid over the last year over the last close.
    #[serde(rename = "dividend_yield")]
    DividendYield,
    /// The average volume of the last 20 days.
    #[serde(rename = "average_volume_20d")]
    AverageVolume20d,
    /// The last day's volume over the average of the last 20 days.
    #[serde(rename = "volume_ratio")]
    VolumeRatio,
    /// The average of the last 20 days' close times volume.
    #[serde(rename = "dollar_volume_20d")]
    DollarVolume20d,
    /// The number of closes up to the date measured to.
    #[serde(rename = "observations")]
    Observations,
    /// The last close on or before the date measured to.
    #[serde(rename = "last_price")]
    LastPrice,
    /// 1 when the last close is more than the days allowed before the date measured to, else 0.
    #[serde(rename = "stale")]
    Stale,
}

/// How a measure is shown: as a percentage, a ratio, a price, an index from 0 to 100, a count or
/// a yes or no.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MeasureUnit {
    /// A fraction shown as a percentage: 0.12 is 12%.
    Fraction,
    /// A ratio, such as a Sharpe ratio or a beta.
    Ratio,
    /// An amount in the units of the price.
    Price,
    /// An index from 0 to 100.
    Index,
    /// A count, such as shares traded.
    Count,
    /// 1 for yes and 0 for no.
    Flag,
}

/// The history a measure needs: a calendar window ending on the date measured to, or a number
/// of closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Need {
    /// A price on or before the window's start.
    Window(Period),
    /// At least this many closes.
    Closes(usize),
}

/// A trailing calendar window ending on the date measured to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Period {
    Week,
    Month,
    Months3,
    Months6,
    YearToDate,
    Year,
    Years3,
    Years5,
}

impl Period {
    pub(crate) const ALL: [Period; 8] = [
        Period::Week,
        Period::Month,
        Period::Months3,
        Period::Months6,
        Period::YearToDate,
        Period::Year,
        Period::Years3,
        Period::Years5,
    ];

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// What a screen needs to know of each measure.
struct Info {
    measure: Measure,
    name: &'static str,
    label: &'static str,
    unit: MeasureUnit,
    group: &'static str,
    need: Need,
}

const fn info(
    measure: Measure,
    name: &'static str,
    label: &'static str,
    unit: MeasureUnit,
    group: &'static str,
    need: Need,
) -> Info {
    Info {
        measure,
        name,
        label,
        unit,
        group,
        need,
    }
}

const RETURNS: &str = "Returns";
const RISK: &str = "Risk";
const BENCHMARK: &str = "Against the benchmark";
const TECHNICALS: &str = "Technicals";
const INCOME: &str = "Income";
const VOLUME: &str = "Volume";
const DATA: &str = "Data";

use MeasureUnit::{Count, Flag, Fraction, Index, Price, Ratio};
use Need::{Closes, Window};

/// Every measure, in the order they are listed and computed by default.
const INFO: [Info; 39] = [
    info(
        Measure::Return1w,
        "return_1w",
        "Return 1w",
        Fraction,
        RETURNS,
        Window(Period::Week),
    ),
    info(
        Measure::Return1m,
        "return_1m",
        "Return 1m",
        Fraction,
        RETURNS,
        Window(Period::Month),
    ),
    info(
        Measure::Return3m,
        "return_3m",
        "Return 3m",
        Fraction,
        RETURNS,
        Window(Period::Months3),
    ),
    info(
        Measure::Return6m,
        "return_6m",
        "Return 6m",
        Fraction,
        RETURNS,
        Window(Period::Months6),
    ),
    info(
        Measure::ReturnYtd,
        "return_ytd",
        "Return YTD",
        Fraction,
        RETURNS,
        Window(Period::YearToDate),
    ),
    info(
        Measure::Return1y,
        "return_1y",
        "Return 1y",
        Fraction,
        RETURNS,
        Window(Period::Year),
    ),
    info(
        Measure::Return3y,
        "return_3y",
        "Return 3y a year",
        Fraction,
        RETURNS,
        Window(Period::Years3),
    ),
    info(
        Measure::Return5y,
        "return_5y",
        "Return 5y a year",
        Fraction,
        RETURNS,
        Window(Period::Years5),
    ),
    info(
        Measure::Volatility,
        "volatility",
        "Volatility",
        Fraction,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::Volatility3y,
        "volatility_3y",
        "Volatility 3y",
        Fraction,
        RISK,
        Window(Period::Years3),
    ),
    info(
        Measure::DownsideDeviation,
        "downside_deviation",
        "Downside deviation",
        Fraction,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::MaxDrawdown,
        "max_drawdown",
        "Max drawdown",
        Fraction,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::MaxDrawdown3y,
        "max_drawdown_3y",
        "Max drawdown 3y",
        Fraction,
        RISK,
        Window(Period::Years3),
    ),
    info(
        Measure::Drawdown,
        "drawdown",
        "Drawdown",
        Fraction,
        RISK,
        Closes(1),
    ),
    info(
        Measure::SharpeRatio,
        "sharpe_ratio",
        "Sharpe ratio",
        Ratio,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::SharpeRatio3y,
        "sharpe_ratio_3y",
        "Sharpe ratio 3y",
        Ratio,
        RISK,
        Window(Period::Years3),
    ),
    info(
        Measure::SortinoRatio,
        "sortino_ratio",
        "Sortino ratio",
        Ratio,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::CalmarRatio,
        "calmar_ratio",
        "Calmar ratio",
        Ratio,
        RISK,
        Window(Period::Years3),
    ),
    info(
        Measure::ValueAtRisk,
        "value_at_risk",
        "Value at risk",
        Fraction,
        RISK,
        Window(Period::Year),
    ),
    info(
        Measure::Beta,
        "beta",
        "Beta",
        Ratio,
        BENCHMARK,
        Window(Period::Year),
    ),
    info(
        Measure::Correlation,
        "correlation",
        "Correlation",
        Ratio,
        BENCHMARK,
        Window(Period::Year),
    ),
    info(
        Measure::RelativeReturn6m,
        "relative_return_6m",
        "Relative return 6m",
        Fraction,
        BENCHMARK,
        Window(Period::Months6),
    ),
    info(
        Measure::RelativeReturn1y,
        "relative_return_1y",
        "Relative return 1y",
        Fraction,
        BENCHMARK,
        Window(Period::Year),
    ),
    info(
        Measure::PriceToSma20,
        "price_to_sma_20",
        "Price to SMA 20",
        Ratio,
        TECHNICALS,
        Closes(20),
    ),
    info(
        Measure::PriceToSma50,
        "price_to_sma_50",
        "Price to SMA 50",
        Ratio,
        TECHNICALS,
        Closes(50),
    ),
    info(
        Measure::PriceToSma200,
        "price_to_sma_200",
        "Price to SMA 200",
        Ratio,
        TECHNICALS,
        Closes(200),
    ),
    info(
        Measure::Sma50Above200,
        "sma_50_above_200",
        "SMA 50 above 200",
        Flag,
        TECHNICALS,
        Closes(200),
    ),
    info(
        Measure::Rsi14,
        "rsi_14",
        "RSI 14",
        Index,
        TECHNICALS,
        Closes(15),
    ),
    info(
        Measure::MacdHistogram,
        "macd_histogram",
        "MACD histogram",
        Price,
        TECHNICALS,
        Closes(MACD_CLOSES),
    ),
    info(
        Measure::From52wHigh,
        "from_52w_high",
        "From 52w high",
        Fraction,
        TECHNICALS,
        Window(Period::Year),
    ),
    info(
        Measure::From52wLow,
        "from_52w_low",
        "From 52w low",
        Fraction,
        TECHNICALS,
        Window(Period::Year),
    ),
    info(
        Measure::Volatility20d,
        "volatility_20d",
        "Volatility 20d",
        Fraction,
        TECHNICALS,
        Closes(21),
    ),
    info(
        Measure::DividendYield,
        "dividend_yield",
        "Dividend yield",
        Fraction,
        INCOME,
        Window(Period::Year),
    ),
    info(
        Measure::AverageVolume20d,
        "average_volume_20d",
        "Average volume 20d",
        Count,
        VOLUME,
        Closes(20),
    ),
    info(
        Measure::VolumeRatio,
        "volume_ratio",
        "Volume ratio",
        Ratio,
        VOLUME,
        Closes(20),
    ),
    info(
        Measure::DollarVolume20d,
        "dollar_volume_20d",
        "Dollar volume 20d",
        Count,
        VOLUME,
        Closes(20),
    ),
    info(
        Measure::Observations,
        "observations",
        "Observations",
        Count,
        DATA,
        Closes(0),
    ),
    info(
        Measure::LastPrice,
        "last_price",
        "Last price",
        Price,
        DATA,
        Closes(1),
    ),
    info(Measure::Stale, "stale", "Stale", Flag, DATA, Closes(0)),
];

/// The closes MACD needs: the 26-day average and 9 values of the line after it.
const MACD_CLOSES: usize = 34;

/// The names that are always given for every security, and that are not measures of their own.
pub(crate) const DATES: [&str; 2] = ["first_date", "last_date"];

impl Measure {
    /// Every measure, in the order a screen lists them.
    pub fn all() -> impl Iterator<Item = Measure> {
        INFO.iter().map(|i| i.measure)
    }

    fn info(self) -> &'static Info {
        // The table holds every measure, in the order of the enumeration.
        &INFO[self as usize]
    }

    /// The measure's name in options and results, such as `"return_1y"`.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// The measure named `name`, such as `"return_1y"`.
    pub fn from_name(name: &str) -> Option<Measure> {
        INFO.iter().find(|i| i.name == name).map(|i| i.measure)
    }

    /// The measure's name in words, for reports, such as `"Return 1y"`.
    pub fn label(self) -> &'static str {
        self.info().label
    }

    /// How the measure is shown.
    pub fn unit(self) -> MeasureUnit {
        self.info().unit
    }

    /// The group the measure belongs to, such as `"Returns"` or `"Technicals"`.
    pub fn group(self) -> &'static str {
        self.info().group
    }

    pub(crate) fn need(self) -> Need {
        self.info().need
    }

    /// Whether the measure is against the benchmark, and so needs one.
    pub fn needs_benchmark(self) -> bool {
        self.group() == BENCHMARK
    }

    /// Whether the measure is of volumes, and so needs them.
    pub fn needs_volume(self) -> bool {
        self.group() == VOLUME
    }

    /// Whether the measure is of dividends, and so needs them.
    pub fn needs_dividends(self) -> bool {
        self.group() == INCOME
    }
}

impl std::fmt::Display for Measure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

// ── Computing ────────────────────────────────────────────────────────────────────────────────

/// What is the same for every security of a screen.
pub(crate) struct Setting<'a> {
    /// The date measured to, as a day number.
    pub(crate) as_of: i32,
    /// The most days the last close may be before `as_of`.
    pub(crate) max_carry_days: i32,
    /// The risk-free rate a day: `(1 + rate)^(1/252) - 1`.
    pub(crate) risk_free: f64,
    /// The day each window starts on, in the order of [`Period::ALL`]: a security covers a window
    /// when it has a close on or before it.
    pub(crate) starts: [i32; 8],
    /// The measures computed, in order.
    pub(crate) measures: &'a [Measure],
    /// The benchmark, when one is given and has a close close enough to `as_of`.
    pub(crate) benchmark: Option<Benchmark<'a>>,
}

/// One security's prices, as day numbers and closes, with its volumes and dividends if given.
#[derive(Clone, Copy)]
pub(crate) struct Prices<'a> {
    pub(crate) days: &'a [i32],
    pub(crate) close: &'a [f64],
    pub(crate) volume: Option<&'a [f64]>,
    pub(crate) dividends: Option<&'a [f64]>,
}

/// The benchmark's prices, with each day's close found in one step.
pub(crate) struct Benchmark<'a> {
    first: i32,
    /// The row of each day from `first`, or `u32::MAX` on a day without a close.
    rows: Vec<u32>,
    close: &'a [f64],
    /// The benchmark's return over each window, when it covers the window.
    returns: [Option<f64>; 8],
}

impl<'a> Benchmark<'a> {
    /// The benchmark of `prices` up to `as_of`, with its return over each window.
    pub(crate) fn new(prices: Prices<'a>, setting_starts: &[i32; 8], as_of: i32) -> Benchmark<'a> {
        let n = prices.days.partition_point(|&d| d <= as_of);
        let days = &prices.days[..n];
        let close = &prices.close[..n];
        let first = days.first().copied().unwrap_or(0);
        let span = days.last().map_or(0, |last| (last - first) as usize + 1);
        let mut rows = vec![u32::MAX; span];
        for (row, d) in days.iter().enumerate() {
            rows[(d - first) as usize] = row as u32;
        }
        let mut returns = [None; 8];
        if let Some(&last) = close.last() {
            for (k, start) in setting_starts.iter().enumerate() {
                let p = days.partition_point(|&d| d <= *start);
                if p > 0 {
                    returns[k] = Some(last / close[p - 1] - 1.0);
                }
            }
        }
        Benchmark {
            first,
            rows,
            close,
            returns,
        }
    }

    /// The benchmark's close on `day`, if it has one.
    fn on(&self, day: i32) -> Option<f64> {
        let offset = usize::try_from(day - self.first).ok()?;
        match self.rows.get(offset) {
            Some(&row) if row != u32::MAX => Some(self.close[row as usize]),
            _ => None,
        }
    }
}

/// The statistics of a window's daily returns.
#[derive(Default)]
struct Stats {
    /// The annualised sample standard deviation; zero for returns that do not vary.
    volatility: Option<f64>,
    sharpe: Option<f64>,
    downside_deviation: Option<f64>,
    sortino: Option<f64>,
    max_drawdown: Option<f64>,
    value_at_risk: Option<f64>,
}

/// The mean and the sample standard deviation of `r`, and whether `r` does not vary, as
/// [`crate::statistics`] decides it.
fn moments(r: &[f64]) -> (f64, f64, bool) {
    let n = r.len() as f64;
    let mean = finalytics_core::numeric::compensated_sum(r.iter().copied()) / n;
    let mut m2 = 0.0;
    let mut scale: f64 = 0.0;
    for &x in r {
        let d = x - mean;
        m2 += d * d;
        scale = scale.max(x.abs());
    }
    let constant = (m2 / n).sqrt() <= 64.0 * f64::EPSILON * scale;
    (mean, (m2 / (n - 1.0)).sqrt(), constant)
}

/// The annualised standard deviation of `r`: zero when `r` does not vary.
fn volatility(r: &[f64]) -> Option<f64> {
    if r.len() < 2 {
        return None;
    }
    let (_, sd, constant) = moments(r);
    Some(if constant {
        0.0
    } else {
        sd * DAYS_A_YEAR.sqrt()
    })
}

/// The quantile of sorted `values` at `p`, interpolated linearly between order statistics.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * p;
    let below = position.floor();
    let k = below as usize;
    match sorted.get(k + 1) {
        Some(next) => sorted[k] + (position - below) * (next - sorted[k]),
        None => sorted[k],
    }
}

/// The statistics of the window whose closes are `close`, from its base, with returns `r`.
fn window(close: &[f64], r: &[f64], risk_free: f64, tail: bool) -> Stats {
    let mut w = Stats::default();
    if let (Some(&base), Some(rest)) = (close.first(), close.get(1..)) {
        let mut peak = base;
        let mut worst: f64 = 0.0;
        for &p in rest {
            peak = peak.max(p);
            worst = worst.min(p / peak - 1.0);
        }
        w.max_drawdown = Some(worst + 0.0);
    }
    if r.len() < 2 {
        return w;
    }
    let (mean, sd, constant) = moments(r);
    let root = DAYS_A_YEAR.sqrt();
    w.volatility = Some(if constant { 0.0 } else { sd * root });
    w.sharpe = (!constant).then(|| (mean - risk_free) / sd * root);
    let shortfall = r
        .iter()
        .map(|x| (x - risk_free).min(0.0).powi(2))
        .sum::<f64>()
        / r.len() as f64;
    let downside = shortfall.sqrt();
    w.downside_deviation = Some(downside * root);
    w.sortino = (downside > 0.0).then(|| (mean - risk_free) * DAYS_A_YEAR / (downside * root));
    if tail {
        let mut sorted = r.to_vec();
        sorted.sort_by(f64::total_cmp);
        w.value_at_risk = Some(-quantile(&sorted, 0.05) + 0.0);
    }
    w
}

/// The averages and indicators taken from the whole history up to the date measured to.
struct Technicals {
    rsi: Option<f64>,
    macd: Option<f64>,
}

/// Wilder's relative strength index of the last 14 changes, and the MACD histogram, of `close`.
fn technicals(close: &[f64], rsi: bool, macd: bool) -> Technicals {
    let mut out = Technicals {
        rsi: None,
        macd: None,
    };
    if rsi && close.len() >= 15 {
        let (mut gain, mut loss) = (0.0, 0.0);
        for pair in close[..15].windows(2) {
            let change = pair[1] - pair[0];
            gain += change.max(0.0);
            loss += (-change).max(0.0);
        }
        gain /= 14.0;
        loss /= 14.0;
        for pair in close[14..].windows(2) {
            let change = pair[1] - pair[0];
            gain = (gain * 13.0 + change.max(0.0)) / 14.0;
            loss = (loss * 13.0 + (-change).max(0.0)) / 14.0;
        }
        out.rsi = (gain + loss > 0.0).then(|| 100.0 * gain / (gain + loss));
    }
    if macd && close.len() >= MACD_CLOSES {
        let (fast, slow, signal) = (2.0 / 13.0, 2.0 / 27.0, 2.0 / 10.0);
        let mut e12 = close[0];
        let mut e26 = close[0];
        let mut line = 0.0;
        let mut average = 0.0;
        for &p in &close[1..] {
            e12 += fast * (p - e12);
            e26 += slow * (p - e26);
            line = e12 - e26;
            average += signal * (line - average);
        }
        out.macd = Some(line - average);
    }
    out
}

/// One security's measures: a value for each measure of the setting (absent where it has none),
/// and whether its history is too short for each.
pub(crate) struct Measured {
    pub(crate) values: Vec<Option<f64>>,
    /// The closes up to the date measured to.
    pub(crate) observations: usize,
    /// Whether the last close is too long before the date measured to.
    pub(crate) stale: bool,
    pub(crate) first_day: Option<i32>,
    pub(crate) last_day: Option<i32>,
}

/// Whether `prices`, up to the date measured to (`n` closes), has the history `measure` needs.
pub(crate) fn covers(measure: Measure, prices: &Prices<'_>, n: usize, starts: &[i32; 8]) -> bool {
    match measure.need() {
        Need::Closes(k) => n >= k,
        Need::Window(period) => n > 0 && prices.days[0] <= starts[period.index()],
    }
}

/// The measures of one security.
pub(crate) fn measure(setting: &Setting<'_>, prices: Prices<'_>) -> Measured {
    let n = prices.days.partition_point(|&d| d <= setting.as_of);
    let days = &prices.days[..n];
    let close = &prices.close[..n];
    let mut out = Measured {
        values: vec![None; setting.measures.len()],
        observations: n,
        stale: false,
        first_day: days.first().copied(),
        last_day: days.last().copied(),
    };
    let Some(&last) = close.last() else {
        for (slot, m) in out.values.iter_mut().zip(setting.measures) {
            if matches!(m, Measure::Observations | Measure::Stale) {
                *slot = Some(0.0);
            }
        }
        return out;
    };
    let k = n - 1;
    out.stale = setting.as_of - days[k] > setting.max_carry_days;
    let wanted = |m: Measure| setting.measures.contains(&m);
    for (slot, m) in out.values.iter_mut().zip(setting.measures) {
        *slot = match m {
            Measure::Observations => Some(n as f64),
            Measure::LastPrice => Some(last),
            Measure::Stale => Some(if out.stale { 1.0 } else { 0.0 }),
            _ => None,
        };
    }
    if out.stale {
        return out;
    }

    // The row each window's returns are measured from: the last close on or before its start.
    let base = |period: Period| -> Option<usize> {
        let p = days.partition_point(|&d| d <= setting.starts[period.index()]);
        p.checked_sub(1)
    };
    let ret = |period: Period| base(period).map(|b| last / close[b] - 1.0);

    // The daily returns from the earliest base a statistic needs, computed once.
    let year = base(Period::Year);
    let three = base(Period::Years3);
    let wants_year = [
        Measure::Volatility,
        Measure::DownsideDeviation,
        Measure::MaxDrawdown,
        Measure::SharpeRatio,
        Measure::SortinoRatio,
        Measure::ValueAtRisk,
    ]
    .into_iter()
    .any(wanted);
    let wants_three = [
        Measure::Volatility3y,
        Measure::MaxDrawdown3y,
        Measure::SharpeRatio3y,
        Measure::CalmarRatio,
    ]
    .into_iter()
    .any(wanted);
    // The row 20 returns before the last.
    let recent = n.checked_sub(21).filter(|_| wanted(Measure::Volatility20d));
    let from = [
        year.filter(|_| wants_year),
        three.filter(|_| wants_three),
        recent,
    ]
    .into_iter()
    .flatten()
    .min();
    let returns: Vec<f64> = match from {
        Some(b) => close[b..].windows(2).map(|p| p[1] / p[0] - 1.0).collect(),
        None => Vec::new(),
    };
    // The returns of the window based at row `b`.
    let since = |b: usize| -> &[f64] {
        match from {
            Some(f) if b >= f => &returns[b - f..],
            _ => &[],
        }
    };
    let rf = setting.risk_free;
    let year_window = match year.filter(|_| wants_year) {
        Some(b) => window(&close[b..], since(b), rf, wanted(Measure::ValueAtRisk)),
        None => Stats::default(),
    };
    let three_window = match three.filter(|_| wants_three) {
        Some(b) => window(&close[b..], since(b), rf, false),
        None => Stats::default(),
    };

    let technicals = technicals(
        close,
        wanted(Measure::Rsi14),
        wanted(Measure::MacdHistogram),
    );
    let sma = |count: usize| -> Option<f64> {
        (n >= count).then(|| close[n - count..].iter().sum::<f64>() / count as f64)
    };
    let year_range = |high: bool| -> Option<f64> {
        let b = year?;
        let fold = if high { f64::max } else { f64::min };
        let start = if high {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        Some(last / close[b..].iter().copied().fold(start, fold) - 1.0)
    };
    let relative = |period: Period| -> Option<f64> {
        let b = setting.benchmark.as_ref()?.returns[period.index()]?;
        Some(ret(period)? - b)
    };
    let paired = || -> (Option<f64>, Option<f64>) {
        let (Some(bench), Some(b)) = (setting.benchmark.as_ref(), year) else {
            return (None, None);
        };
        let pairs: Vec<(f64, f64)> = (b..n)
            .filter_map(|j| bench.on(days[j]).map(|q| (close[j], q)))
            .collect();
        let (rs, rb): (Vec<f64>, Vec<f64>) = pairs
            .windows(2)
            .map(|w| (w[1].0 / w[0].0 - 1.0, w[1].1 / w[0].1 - 1.0))
            .unzip();
        if rs.len() < 2 {
            return (None, None);
        }
        let (ms, _, cs) = moments(&rs);
        let (mb, _, cb) = moments(&rb);
        let (mut cov, mut vs, mut vb) = (0.0, 0.0, 0.0);
        for (x, y) in rs.iter().zip(&rb) {
            cov += (x - ms) * (y - mb);
            vs += (x - ms) * (x - ms);
            vb += (y - mb) * (y - mb);
        }
        let beta = (!cb).then(|| cov / vb);
        let correlation = (!cb && !cs).then(|| cov / (vs * vb).sqrt());
        (beta, correlation)
    };
    let (beta, correlation) = if wanted(Measure::Beta) || wanted(Measure::Correlation) {
        paired()
    } else {
        (None, None)
    };
    let recent_volumes = || -> Option<&[f64]> {
        let v = prices.volume?;
        (n >= 20)
            .then(|| &v[n - 20..n])
            .filter(|w| w.iter().all(|x| x.is_finite()))
    };

    for (slot, m) in out.values.iter_mut().zip(setting.measures) {
        if slot.is_some() {
            continue;
        }
        *slot = match m {
            Measure::Return1w => ret(Period::Week),
            Measure::Return1m => ret(Period::Month),
            Measure::Return3m => ret(Period::Months3),
            Measure::Return6m => ret(Period::Months6),
            Measure::ReturnYtd => ret(Period::YearToDate),
            Measure::Return1y => ret(Period::Year),
            Measure::Return3y => ret(Period::Years3).map(|r| annualise(r, 3.0)),
            Measure::Return5y => ret(Period::Years5).map(|r| annualise(r, 5.0)),
            Measure::Volatility => year_window.volatility,
            Measure::Volatility3y => three_window.volatility,
            Measure::DownsideDeviation => year_window.downside_deviation,
            Measure::MaxDrawdown => year_window.max_drawdown,
            Measure::MaxDrawdown3y => three_window.max_drawdown,
            Measure::Drawdown => {
                let peak = close.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                Some(last / peak - 1.0 + 0.0)
            }
            Measure::SharpeRatio => year_window.sharpe,
            Measure::SharpeRatio3y => three_window.sharpe,
            Measure::SortinoRatio => year_window.sortino,
            Measure::CalmarRatio => match (ret(Period::Years3), three_window.max_drawdown) {
                (Some(r), Some(dd)) if dd < 0.0 => Some(annualise(r, 3.0) / -dd),
                _ => None,
            },
            Measure::ValueAtRisk => year_window.value_at_risk,
            Measure::Beta => beta,
            Measure::Correlation => correlation,
            Measure::RelativeReturn6m => relative(Period::Months6),
            Measure::RelativeReturn1y => relative(Period::Year),
            Measure::PriceToSma20 => sma(20).map(|s| last / s),
            Measure::PriceToSma50 => sma(50).map(|s| last / s),
            Measure::PriceToSma200 => sma(200).map(|s| last / s),
            Measure::Sma50Above200 => match (sma(50), sma(200)) {
                (Some(a), Some(b)) => Some(if a > b { 1.0 } else { 0.0 }),
                _ => None,
            },
            Measure::Rsi14 => technicals.rsi,
            Measure::MacdHistogram => technicals.macd,
            Measure::From52wHigh => year_range(true).map(|x| x + 0.0),
            Measure::From52wLow => year_range(false).map(|x| x + 0.0),
            Measure::Volatility20d => recent.and_then(|b| volatility(since(b))),
            Measure::DividendYield => match (prices.dividends, year) {
                (Some(d), Some(_)) => {
                    let start = setting.starts[Period::Year.index()];
                    let first = days.partition_point(|&day| day <= start);
                    Some(d[first..n].iter().sum::<f64>() / last)
                }
                _ => None,
            },
            Measure::AverageVolume20d => {
                recent_volumes().map(|v| v.iter().sum::<f64>() / v.len() as f64)
            }
            Measure::VolumeRatio => recent_volumes().and_then(|v| {
                let average = v.iter().sum::<f64>() / v.len() as f64;
                (average > 0.0).then(|| v[v.len() - 1] / average)
            }),
            Measure::DollarVolume20d => recent_volumes().map(|v| {
                v.iter()
                    .zip(&close[n - 20..])
                    .map(|(v, p)| v * p)
                    .sum::<f64>()
                    / 20.0
            }),
            Measure::Observations | Measure::LastPrice | Measure::Stale => None,
        };
    }
    out
}

/// A return over `years` as a return a year.
fn annualise(r: f64, years: f64) -> f64 {
    (1.0 + r).powf(1.0 / years) - 1.0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::{StatisticsOptions, TailMethod, statistics, tail_risk};

    #[test]
    fn names_agree_with_serde_and_the_table_follows_the_enumeration() {
        for (k, m) in Measure::all().enumerate() {
            assert_eq!(m as usize, k, "{m:?}");
            let named = serde_json::to_value(m).unwrap();
            assert_eq!(named, serde_json::json!(m.name()));
            assert_eq!(Measure::from_name(m.name()), Some(m));
        }
        assert_eq!(Measure::from_name("return_2y"), None);
    }

    #[test]
    fn a_window_agrees_with_the_statistics() {
        let r = [0.01, -0.02, 0.015, 0.003, -0.007, 0.012, -0.004, 0.0];
        let mut close = vec![100.0];
        for x in r {
            close.push(close[close.len() - 1] * (1.0 + x));
        }
        let rf = 0.03;
        let daily = (1.0f64 + rf).powf(1.0 / 252.0) - 1.0;
        let w = window(&close, &r, daily, true);
        let options = StatisticsOptions {
            risk_free_rate: rf,
            target_return: rf,
            ..StatisticsOptions::default()
        };
        let s = statistics(&r, &options).unwrap();
        let close_to =
            |a: Option<f64>, b: f64| assert!((a.unwrap() - b).abs() < 1e-12, "{a:?} {b}");
        close_to(w.volatility, s.volatility);
        close_to(w.sharpe, s.sharpe_ratio.unwrap());
        close_to(w.downside_deviation, s.downside_deviation);
        close_to(w.sortino, s.sortino_ratio.unwrap());
        close_to(w.max_drawdown, s.max_drawdown);
        let t = tail_risk(&r, 0.95, TailMethod::Historical).unwrap();
        close_to(w.value_at_risk, t.value_at_risk);
    }

    #[test]
    fn rsi_and_macd_of_a_steady_rise() {
        let close: Vec<f64> = (0..60).map(|k| 100.0 + f64::from(k)).collect();
        let t = technicals(&close, true, true);
        assert_eq!(t.rsi, Some(100.0));
        assert!(t.macd.unwrap() > 0.0);
        let flat = vec![50.0; 40];
        let t = technicals(&flat, true, true);
        assert_eq!(t.rsi, None);
        assert_eq!(t.macd, Some(0.0));
        assert!(technicals(&close[..14], true, true).rsi.is_none());
        assert!(technicals(&close[..33], true, true).macd.is_none());
    }
}
