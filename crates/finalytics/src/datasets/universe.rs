//! The synthetic universe of [`super::universe`]: 200 fictional companies in eleven sectors and a
//! market index, on every weekday from November 2020 to October 2026.
//!
//! Each day's returns are drawn from a market factor whose volatility changes from day to day
//! (GARCH(1, 1)), a factor for each sector and each company's own risk, with rare jumps. The
//! market falls through 2022 and sells off in the spring of 2025, loosely as markets did then.
//! Every number comes from the seeded generator of `finalytics_core::random` and from `libm`, so
//! every platform, WebAssembly included, draws the same universe.

use std::collections::BTreeMap;

use finalytics_core::random::Random;
use finalytics_core::{Date, Result, Weekday};
use finalytics_risk::screening::{FieldValue, Series};

/// The generator's seed.
const SEED: u64 = 9;
/// The market index, the universe's benchmark.
pub(super) const BENCHMARK: &str = "INDEX";
/// The universe's annual risk-free rate, for Sharpe and Sortino ratios.
pub(super) const RISK_FREE_RATE: f64 = 0.04;
/// The fields, in the order they are shown.
pub(super) const FIELDS: [&str; 5] = [
    "name",
    "sector",
    "market_cap",
    "trailing_pe",
    "price_to_book",
];

const YEAR: f64 = 252.0;

/// A sector: how many companies it has, their betas, its own factor's volatility and drift,
/// the range of its companies' own volatility, how many pay dividends and at what yield, the
/// range of their price-to-earnings and price-to-book ratios, and the words that end their names.
struct Sector {
    name: &'static str,
    count: usize,
    beta: f64,
    volatility: f64,
    drift: f64,
    own_volatility: (f64, f64),
    payers: f64,
    dividend_yield: (f64, f64),
    pe: (f64, f64),
    price_to_book: (f64, f64),
    words: [&'static str; 5],
}

const SECTORS: [Sector; 11] = [
    Sector {
        name: "Technology",
        count: 28,
        beta: 1.25,
        volatility: 0.12,
        drift: 0.04,
        own_volatility: (0.22, 0.45),
        payers: 0.35,
        dividend_yield: (0.004, 0.015),
        pe: (18.0, 45.0),
        price_to_book: (4.0, 14.0),
        words: ["Systems", "Software", "Semiconductor", "Networks", "Data"],
    },
    Sector {
        name: "Health Care",
        count: 24,
        beta: 0.85,
        volatility: 0.10,
        drift: 0.01,
        own_volatility: (0.18, 0.40),
        payers: 0.5,
        dividend_yield: (0.008, 0.025),
        pe: (14.0, 32.0),
        price_to_book: (2.5, 8.0),
        words: [
            "Health",
            "Therapeutics",
            "Medical",
            "Biosciences",
            "Diagnostics",
        ],
    },
    Sector {
        name: "Financials",
        count: 24,
        beta: 1.10,
        volatility: 0.10,
        drift: 0.0,
        own_volatility: (0.15, 0.30),
        payers: 0.85,
        dividend_yield: (0.015, 0.04),
        pe: (9.0, 17.0),
        price_to_book: (0.8, 2.5),
        words: ["Bancorp", "Financial", "Capital", "Insurance", "Trust"],
    },
    Sector {
        name: "Consumer Discretionary",
        count: 20,
        beta: 1.15,
        volatility: 0.11,
        drift: 0.0,
        own_volatility: (0.20, 0.40),
        payers: 0.45,
        dividend_yield: (0.005, 0.02),
        pe: (14.0, 35.0),
        price_to_book: (2.0, 9.0),
        words: ["Brands", "Retail", "Motors", "Home", "Apparel"],
    },
    Sector {
        name: "Industrials",
        count: 22,
        beta: 1.05,
        volatility: 0.09,
        drift: 0.01,
        own_volatility: (0.15, 0.32),
        payers: 0.75,
        dividend_yield: (0.01, 0.025),
        pe: (14.0, 26.0),
        price_to_book: (2.0, 6.0),
        words: ["Industries", "Aerospace", "Logistics", "Machinery", "Rail"],
    },
    Sector {
        name: "Communication Services",
        count: 14,
        beta: 1.0,
        volatility: 0.11,
        drift: 0.0,
        own_volatility: (0.20, 0.40),
        payers: 0.4,
        dividend_yield: (0.005, 0.03),
        pe: (12.0, 30.0),
        price_to_book: (1.5, 6.0),
        words: [
            "Media",
            "Communications",
            "Telecom",
            "Studios",
            "Interactive",
        ],
    },
    Sector {
        name: "Consumer Staples",
        count: 16,
        beta: 0.6,
        volatility: 0.07,
        drift: -0.01,
        own_volatility: (0.12, 0.24),
        payers: 0.9,
        dividend_yield: (0.02, 0.04),
        pe: (16.0, 26.0),
        price_to_book: (2.5, 8.0),
        words: ["Foods", "Beverages", "Household", "Farms", "Grocers"],
    },
    Sector {
        name: "Energy",
        count: 14,
        beta: 1.0,
        volatility: 0.20,
        drift: 0.02,
        own_volatility: (0.22, 0.38),
        payers: 0.85,
        dividend_yield: (0.025, 0.06),
        pe: (7.0, 15.0),
        price_to_book: (1.0, 2.5),
        words: ["Energy", "Petroleum", "Midstream", "Drilling", "Resources"],
    },
    Sector {
        name: "Utilities",
        count: 12,
        beta: 0.5,
        volatility: 0.08,
        drift: -0.01,
        own_volatility: (0.12, 0.20),
        payers: 1.0,
        dividend_yield: (0.03, 0.05),
        pe: (14.0, 22.0),
        price_to_book: (1.2, 2.5),
        words: ["Power", "Utilities", "Water", "Electric", "Gas"],
    },
    Sector {
        name: "Materials",
        count: 12,
        beta: 1.05,
        volatility: 0.12,
        drift: 0.0,
        own_volatility: (0.18, 0.35),
        payers: 0.7,
        dividend_yield: (0.012, 0.035),
        pe: (10.0, 22.0),
        price_to_book: (1.2, 4.0),
        words: ["Materials", "Chemicals", "Steel", "Mining", "Packaging"],
    },
    Sector {
        name: "Real Estate",
        count: 14,
        beta: 0.85,
        volatility: 0.12,
        drift: -0.02,
        own_volatility: (0.15, 0.28),
        payers: 0.95,
        dividend_yield: (0.03, 0.06),
        pe: (20.0, 45.0),
        price_to_book: (1.0, 3.0),
        words: ["Properties", "Realty", "Land", "Towers", "Estates"],
    },
];

/// The first and second parts of the companies' names: each pair names one company, and the
/// first four letters of the pair are its identifier, so no two companies share one.
const FIRST: [&str; 25] = [
    "Ald", "Bra", "Cal", "Dun", "Elm", "Fal", "Gar", "Hal", "Ivo", "Kel", "Lin", "Mar", "Nor",
    "Osb", "Pem", "Qui", "Ros", "Sel", "Tor", "Ulv", "Ver", "Wex", "Yar", "Zen", "Ash",
];
const SECOND: [&str; 8] = [
    "by", "combe", "dale", "garth", "holm", "mere", "stead", "wick",
];

/// How many companies list after the universe starts, and the last close of the one that stops
/// trading.
const LISTED_LATE: usize = 14;
const STALE_FROM: (i32, u32, u32) = (2026, 8, 14);

/// A uniform number between `low` and `high`.
fn between(random: &mut Random, (low, high): (f64, f64)) -> f64 {
    low + (high - low) * random.uniform()
}

/// A number between `low` and `high` whose logarithm is uniform.
fn log_between(random: &mut Random, low: f64, high: f64) -> f64 {
    libm::exp(libm::log(low) + (libm::log(high) - libm::log(low)) * random.uniform())
}

/// `x` rounded to `places` decimal places.
fn rounded(x: f64, places: i32) -> f64 {
    let scale = libm::pow(10.0, f64::from(places));
    libm::round(x * scale) / scale
}

/// Every weekday from `first` to `last`.
fn weekdays(first: Date, last: Date) -> Result<Vec<Date>> {
    let mut days = Vec::new();
    let mut day = first;
    while day <= last {
        if !matches!(day.weekday(), Weekday::Saturday | Weekday::Sunday) {
            days.push(day);
        }
        day = day.add_days(1)?;
    }
    Ok(days)
}

/// The market's extra annual drift on `day`: the fall of 2022 and the sell-off of spring 2025.
fn episode(day: Date) -> f64 {
    let (y, m, d) = day.ymd();
    match (y, m) {
        (2022, 1..=9) => -0.32,
        (2022, 10) if d <= 14 => -0.32,
        (2025, 3) => -0.45,
        (2025, 4) if d <= 8 => -1.6,
        _ => 0.0,
    }
}

/// The market's daily returns: GARCH(1, 1) volatility around 17% a year, with the episodes.
fn market_returns(random: &mut Random, days: &[Date]) -> Vec<f64> {
    let long_run = 0.17 * 0.17 / YEAR;
    let (alpha, beta) = (0.09, 0.89);
    let omega = long_run * (1.0 - alpha - beta);
    let mut variance = long_run;
    let mut shock: f64 = 0.0;
    days.iter()
        .map(|&day| {
            variance = omega + alpha * shock * shock + beta * variance;
            shock = libm::sqrt(variance) * random.normal();
            // The episodes are turbulent: their returns move half again as much.
            let scale = if episode(day) < 0.0 { 1.5 } else { 1.0 };
            (0.10 + episode(day)) / YEAR + scale * shock
        })
        .collect()
}

/// A company: its identifier, fields and the parameters of its returns.
struct Company {
    id: String,
    fields: BTreeMap<String, FieldValue>,
    sector: usize,
    beta: f64,
    alpha: f64,
    own_volatility: f64,
    start_price: f64,
    dividend_yield: f64,
    dollar_volume: f64,
}

/// The names of the companies, one first part and second part each, in a shuffled order.
fn name_parts(random: &mut Random) -> Vec<(usize, usize)> {
    let mut parts: Vec<(usize, usize)> = (0..FIRST.len())
        .flat_map(|i| (0..SECOND.len()).map(move |j| (i, j)))
        .collect();
    for k in (1..parts.len()).rev() {
        parts.swap(k, random.below(k + 1));
    }
    parts
}

fn companies(random: &mut Random) -> Vec<Company> {
    let parts = name_parts(random);
    let mut found = Vec::new();
    for (s, sector) in SECTORS.iter().enumerate() {
        for k in 0..sector.count {
            let (i, j) = parts[found.len()];
            let stem = format!("{}{}", FIRST[i], SECOND[j]);
            let id = stem[..4].to_uppercase();
            let name = format!("{stem} {}", sector.words[k % sector.words.len()]);
            let beta = (sector.beta + 0.25 * random.normal()).clamp(0.3, 2.0);
            let alpha = 0.04 * random.normal();
            let own_volatility = between(random, sector.own_volatility);
            let start_price = rounded(log_between(random, 8.0, 320.0), 2);
            let pays = random.uniform() < sector.payers;
            let dividend_yield = if pays {
                between(random, sector.dividend_yield)
            } else {
                0.0
            };
            let market_cap = rounded(log_between(random, 1.5e9, 6.0e11) / 1e6, 0) * 1e6;
            let dollar_volume = market_cap * between(random, (0.002, 0.008));
            // One company in twelve makes a loss, and has no price-to-earnings ratio.
            let pe = between(random, sector.pe);
            let loss = random.uniform() < 1.0 / 12.0;
            let price_to_book = between(random, sector.price_to_book);
            let mut fields = BTreeMap::new();
            fields.insert("name".to_string(), FieldValue::Text(name));
            fields.insert(
                "sector".to_string(),
                FieldValue::Text(sector.name.to_string()),
            );
            fields.insert("market_cap".to_string(), FieldValue::Number(market_cap));
            if !loss {
                fields.insert(
                    "trailing_pe".to_string(),
                    FieldValue::Number(rounded(pe, 1)),
                );
            }
            fields.insert(
                "price_to_book".to_string(),
                FieldValue::Number(rounded(price_to_book, 2)),
            );
            found.push(Company {
                id,
                fields,
                sector: s,
                beta,
                alpha,
                own_volatility,
                start_price,
                dividend_yield,
                dollar_volume,
            });
        }
    }
    found
}

/// The universe's series: the 200 companies, then the market index.
pub(super) fn series() -> Result<Vec<Series>> {
    let mut random = Random::new(SEED);
    let days = weekdays(Date::from_ymd(2020, 11, 2)?, Date::from_ymd(2026, 10, 30)?)?;
    let stale_from = Date::from_ymd(STALE_FROM.0, STALE_FROM.1, STALE_FROM.2)?;
    let market = market_returns(&mut random, &days);
    let sectors: Vec<Vec<f64>> = SECTORS
        .iter()
        .map(|s| {
            days.iter()
                .map(|_| s.drift / YEAR + s.volatility / libm::sqrt(YEAR) * random.normal())
                .collect()
        })
        .collect();
    let companies = companies(&mut random);
    let n = companies.len();

    // Which companies list late, which stops trading, which miss days and which have no volumes:
    // chosen by the generator, from different companies.
    let mut order: Vec<usize> = (0..n).collect();
    for k in (1..n).rev() {
        order.swap(k, random.below(k + 1));
    }
    let late = &order[..LISTED_LATE];
    let stale = order[LISTED_LATE];
    let gaps = &order[LISTED_LATE + 1..LISTED_LATE + 3];
    let no_volume = &order[LISTED_LATE + 3..LISTED_LATE + 6];

    let mut series = Vec::with_capacity(n + 1);
    for (c, company) in companies.into_iter().enumerate() {
        // A late listing starts between March 2022 and the summer of 2025, so that it has a year
        // of closes, and some have three.
        let first = if let Some(k) = late.iter().position(|&l| l == c) {
            let span = days.len() - 330 - 345;
            345 + (k * span) / LISTED_LATE + random.below(span / LISTED_LATE)
        } else {
            0
        };
        let daily_own = company.own_volatility / libm::sqrt(YEAR);
        // The company's own and its sector's risk add no drag: its growth follows the market's
        // through its beta, with its own alpha.
        let sector = &SECTORS[company.sector];
        let drag = 0.5
            * (company.own_volatility * company.own_volatility
                + sector.volatility * sector.volatility)
            / YEAR;
        let mut price = company.start_price;
        let mut dates = Vec::new();
        let mut close = Vec::new();
        let mut volume = Vec::new();
        let mut dividends = Vec::new();
        for (t, &day) in days.iter().enumerate() {
            let jump = if random.uniform() < 0.002 {
                0.08 * random.normal()
            } else {
                0.0
            };
            let own = daily_own * random.normal() + jump;
            let r = company.alpha / YEAR
                + drag
                + company.beta * market[t]
                + sectors[company.sector][t]
                + own;
            let noise = random.normal();
            if t < first || (c == stale && day > stale_from) {
                continue;
            }
            if t > first {
                price *= 1.0 + r.max(-0.6);
            }
            let missing = gaps.contains(&c) && random.uniform() < 0.02;
            dates.push(day);
            close.push(if missing { f64::NAN } else { rounded(price, 2) });
            // Busier on days that move: the dollar volume over the price, in whole shares.
            let busy = libm::exp(0.35 * noise) * (1.0 + 25.0 * r.abs());
            volume.push(libm::round(company.dollar_volume * busy / price));
            // Dividends quarterly, on the third Friday of February, May, August and November.
            let (_, month, d) = day.ymd();
            let paid = company.dividend_yield > 0.0
                && month % 3 == 2
                && day.weekday() == Weekday::Friday
                && (15..=21).contains(&d)
                && t > first;
            dividends.push(if paid {
                rounded(price * company.dividend_yield / 4.0, 4)
            } else {
                0.0
            });
        }
        let mut found = Series::new(company.id, dates, close).with_dividends(dividends);
        if !no_volume.contains(&c) {
            found = found.with_volume(volume);
        }
        found.fields = company.fields;
        series.push(found);
    }

    // The market index, from 1,000, with its own fields.
    let mut level = 1000.0;
    let close = market
        .iter()
        .enumerate()
        .map(|(t, r)| {
            if t > 0 {
                level *= 1.0 + r;
            }
            rounded(level, 2)
        })
        .collect();
    let index = Series::new(BENCHMARK, days, close).with_field("name", "Market index");
    series.push(index);
    Ok(series)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a over the bits of every number, so that a change anywhere shows.
    fn digest(series: &[Series]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut add = |x: f64| {
            for byte in x.to_bits().to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
            }
        };
        for s in series {
            for d in &s.dates {
                add(f64::from(d.days_since_epoch()));
            }
            s.close.iter().for_each(|&x| add(x));
            s.volume.iter().flatten().for_each(|&x| add(x));
            s.dividends.iter().flatten().for_each(|&x| add(x));
        }
        hash
    }

    #[test]
    fn the_universe_is_the_same_on_every_platform() {
        let series = series().unwrap();
        assert_eq!(series.len(), 201);
        let index = series.last().unwrap();
        assert_eq!(index.id, BENCHMARK);
        assert_eq!(index.dates.len(), 1565);
        assert_eq!(index.close.last(), Some(&1558.67));
        // The same on Linux, macOS, Windows and WebAssembly: only libm and the seeded generator
        // draw it.
        assert_eq!(digest(&series), 15_874_778_401_675_448_106);
    }

    #[test]
    fn the_universe_has_its_irregular_companies() {
        let series = series().unwrap();
        let last = Date::from_ymd(2026, 10, 30).unwrap();
        let first = Date::from_ymd(2020, 11, 2).unwrap();
        let late = series.iter().filter(|s| s.dates[0] > first).count();
        let stale = series
            .iter()
            .filter(|s| *s.dates.last().unwrap() < last)
            .count();
        let gaps = series
            .iter()
            .filter(|s| s.close.iter().any(|c| c.is_nan()))
            .count();
        let no_volume = series.iter().filter(|s| s.volume.is_none()).count();
        assert_eq!((late, stale, gaps, no_volume), (LISTED_LATE, 1, 2, 4));
        let ids: std::collections::BTreeSet<&str> = series.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids.len(), 201);
        let losses = series
            .iter()
            .filter(|s| s.id != BENCHMARK && !s.fields.contains_key("trailing_pe"))
            .count();
        assert!((10..=25).contains(&losses));
    }
}
