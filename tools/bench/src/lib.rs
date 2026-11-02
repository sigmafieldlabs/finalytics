//! Deterministic inputs for the benchmarks and the performance budgets.
//!
//! [`large_portfolio`] builds the portfolio of the performance budget: 2,000 instruments in
//! three currencies, ten accounts, and 100,000 transactions over ten years, with purchases,
//! sales by several lot methods, dividends, deposits and currency conversions. Every input is
//! generated from a fixed seed, so each run measures the same work.

use finalytics::opt::{Constraints, GroupLimit, Problem};
use finalytics::risk::{Matrix, ReturnsTable};
use finalytics::{
    Account, AccountId, Currency, Date, Decimal, Instrument, InstrumentId, InstrumentKind,
    LotMethod, MarketData, Money, Portfolio, TimeSeries, Transaction, TransactionId,
    TransactionKind,
};

/// A small, fast, deterministic random number generator (SplitMix64).
pub struct Random(u64);

impl Random {
    /// A generator from `seed`.
    pub fn new(seed: u64) -> Random {
        Random(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `[0, 1)`.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A whole number below `n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// A standard normal number (Box–Muller).
    pub fn normal(&mut self) -> f64 {
        let u = self.uniform().max(1e-300);
        let v = self.uniform();
        (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
    }
}

/// The business days (Monday to Friday) from `start`, `count` of them.
pub fn business_days(start: Date, count: usize) -> Vec<Date> {
    let mut days = Vec::with_capacity(count);
    let mut day = start;
    while days.len() < count {
        if (day.days_since_epoch() + 3).rem_euclid(7) < 5 {
            days.push(day);
        }
        day = day.add_days(1).unwrap_or(day);
    }
    days
}

fn id<T>(make: fn(String) -> Option<T>, text: String) -> T {
    make(text).unwrap_or_else(|| unreachable!("generated identifiers are valid"))
}

fn decimal(value: f64, places: u32) -> Decimal {
    Decimal::try_from(value)
        .unwrap_or_default()
        .round_dp(places)
}

/// A portfolio of the budget's size and the market data to value it.
pub struct LargePortfolio {
    /// The portfolio.
    pub portfolio: Portfolio,
    /// Daily FX rates and month-end prices for every instrument.
    pub market: MarketData,
    /// The month ends of the ten years, for valuation.
    pub month_ends: Vec<Date>,
}

/// Builds a portfolio with `instruments` instruments and `transactions` transactions over
/// `years` years of business days.
pub fn large_portfolio(instruments: usize, transactions: usize, years: usize) -> LargePortfolio {
    let mut random = Random::new(20_261_102);
    let start = Date::from_ymd(2016, 1, 4).unwrap_or_else(|_| unreachable!());
    let days = business_days(start, years * 261);
    let currencies = [Currency::USD, Currency::EUR, Currency::GBP];

    let ids: Vec<InstrumentId> = (0..instruments)
        .map(|i| id(InstrumentId::new, format!("I{i:04}")))
        .collect();
    let currency_of = |i: usize| match i % 10 {
        0..=6 => Currency::USD,
        7 | 8 => Currency::EUR,
        _ => Currency::GBP,
    };
    let instrument_list: Vec<Instrument> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let kind = match i % 5 {
                0 => InstrumentKind::Fund,
                1 => InstrumentKind::Bond,
                _ => InstrumentKind::Equity,
            };
            Instrument::new(id.clone(), kind, currency_of(i)).classified(
                "asset_class",
                if i % 5 == 1 { "Fixed income" } else { "Equity" },
            )
        })
        .collect();
    let base_price: Vec<f64> = (0..instruments)
        .map(|_| 20.0 + 180.0 * random.uniform())
        .collect();
    let price_on = |i: usize, t: usize| {
        base_price[i] * (1.0 + 0.0002 * t as f64) * (1.0 + 0.1 * ((t + i) as f64 / 50.0).sin())
    };

    let methods = [
        LotMethod::Fifo,
        LotMethod::HighestCost,
        LotMethod::AverageCost,
        LotMethod::Lifo,
    ];
    let accounts: Vec<Account> = (0..10)
        .map(|a| {
            let mut account = Account::new(id(AccountId::new, format!("A{a}")));
            account.lot_method = methods[a % methods.len()];
            account
        })
        .collect();

    let mut held = vec![vec![0u64; instruments]; accounts.len()];
    let mut list = Vec::with_capacity(transactions);
    let push = |list: &mut Vec<Transaction>, account: usize, date: Date, kind: TransactionKind| {
        let n = list.len();
        list.push(Transaction {
            id: id(TransactionId::new, format!("T{n}")),
            account: accounts[account].id.clone(),
            trade_date: date,
            settle_date: None,
            kind,
            note: None,
        });
    };
    // Fund every account in every currency at the start.
    for a in 0..accounts.len() {
        push(
            &mut list,
            a,
            days[0],
            TransactionKind::Deposit {
                amount: Money::new(Decimal::from(500_000_000i64), Currency::USD),
            },
        );
        for currency in &currencies[1..] {
            push(
                &mut list,
                a,
                days[0],
                TransactionKind::FxConversion {
                    sell: Money::new(Decimal::from(100_000_000i64), Currency::USD),
                    buy: Money::new(Decimal::from(80_000_000i64), *currency),
                },
            );
        }
    }
    let per_day = transactions as f64 / days.len() as f64;
    let mut t = 0usize;
    while list.len() < transactions {
        let day = (list.len() as f64 / per_day) as usize;
        t = t.max(day.min(days.len() - 1));
        let date = days[t];
        let a = random.below(accounts.len());
        let i = random.below(instruments);
        let price = decimal(price_on(i, t), 2);
        let roll = random.uniform();
        let kind = if roll < 0.08 && held[a][i] > 0 {
            TransactionKind::Dividend {
                instrument: ids[i].clone(),
                amount: Money::new(decimal(held[a][i] as f64 * 0.25, 2), currency_of(i)),
                withholding_tax: Decimal::ZERO,
            }
        } else if roll < 0.35 && held[a][i] > 1 {
            let quantity = 1 + random.below(held[a][i] as usize / 2 + 1) as u64;
            let quantity = quantity.min(held[a][i]);
            held[a][i] -= quantity;
            TransactionKind::Sell {
                instrument: ids[i].clone(),
                quantity: Decimal::from(quantity),
                price,
                fees: Decimal::new(5, 0),
                lot_method: None,
                lots: Vec::new(),
            }
        } else if roll < 0.37 {
            TransactionKind::Deposit {
                amount: Money::new(Decimal::from(1_000_000i64), Currency::USD),
            }
        } else {
            let quantity = 1 + random.below(200) as u64;
            held[a][i] += quantity;
            TransactionKind::Buy {
                instrument: ids[i].clone(),
                quantity: Decimal::from(quantity),
                price,
                fees: Decimal::new(5, 0),
            }
        };
        push(&mut list, a, date, kind);
    }

    let mut market = MarketData::new();
    let mut eur = 1.10;
    let mut gbp = 1.30;
    let (mut eur_rates, mut gbp_rates) = (Vec::new(), Vec::new());
    for _ in &days {
        eur *= (0.004 * random.normal()).exp();
        gbp *= (0.004 * random.normal()).exp();
        eur_rates.push(decimal(eur, 5));
        gbp_rates.push(decimal(gbp, 5));
    }
    for (currency, rates) in [(Currency::EUR, eur_rates), (Currency::GBP, gbp_rates)] {
        if let Ok(series) = TimeSeries::new(days.clone(), rates) {
            market = market.with_fx(currency, Currency::USD, series);
        }
    }
    let month_ends: Vec<Date> = days
        .windows(2)
        .filter(|w| w[0].month() != w[1].month())
        .map(|w| w[0])
        .collect();
    let month_index: Vec<usize> = month_ends
        .iter()
        .map(|d| days.partition_point(|x| x < d))
        .collect();
    for (i, id) in ids.iter().enumerate() {
        let values = month_index
            .iter()
            .map(|&t| decimal(price_on(i, t), 2))
            .collect();
        if let Ok(series) = TimeSeries::new(month_ends.clone(), values) {
            market = market.with_prices(id.clone(), series);
        }
    }
    LargePortfolio {
        portfolio: Portfolio {
            base_currency: Currency::USD,
            accounts,
            instruments: instrument_list,
            transactions: list,
        },
        market: market.with_max_carry_days(10),
        month_ends,
    }
}

/// Returns for `assets` assets over `periods` periods from a three-factor model, as a table.
pub fn factor_returns(assets: usize, periods: usize) -> ReturnsTable {
    let mut random = Random::new(7);
    let loadings: Vec<[f64; 3]> = (0..assets)
        .map(|_| {
            [
                0.5 + random.uniform(),
                random.normal() * 0.5,
                random.normal() * 0.3,
            ]
        })
        .collect();
    let specific: Vec<f64> = (0..assets)
        .map(|_| 0.005 + 0.01 * random.uniform())
        .collect();
    let mut columns = vec![Vec::with_capacity(periods); assets];
    for _ in 0..periods {
        let f = [
            0.01 * random.normal(),
            0.006 * random.normal(),
            0.004 * random.normal(),
        ];
        for (k, column) in columns.iter_mut().enumerate() {
            let l = loadings[k];
            column.push(
                0.0003 + l[0] * f[0] + l[1] * f[1] + l[2] * f[2] + specific[k] * random.normal(),
            );
        }
    }
    let labels = (0..assets).map(|i| format!("asset {i}")).collect();
    ReturnsTable::new(labels, columns)
        .unwrap_or_else(|e| unreachable!("generated returns are valid: {e}"))
}

/// The budget's optimisation problem: `assets` assets, an annualised covariance estimated with
/// Ledoit–Wolf shrinkage from two years of daily factor returns, expected returns, bounds and
/// ten group limits.
pub fn large_problem(assets: usize) -> Problem {
    let returns = factor_returns(assets, 504);
    let estimate =
        finalytics::risk::covariance(&returns, finalytics::risk::CovarianceMethod::LedoitWolf)
            .unwrap_or_else(|e| unreachable!("the generated covariance is valid: {e}"));
    let covariance: Matrix = estimate.matrix.scaled(252.0);
    with_budget_constraints(Problem::new(covariance), assets)
}

/// The budget's optimisation problem with risk from a ten-factor statistical model of the same
/// returns, annualised, in place of the covariance matrix.
pub fn large_factor_problem(assets: usize) -> Problem {
    let returns = factor_returns(assets, 504);
    let model = finalytics::risk::statistical_model(&returns, 10)
        .unwrap_or_else(|e| unreachable!("the generated returns have components: {e}"))
        .model;
    with_budget_constraints(Problem::from_factor_model(model.scaled(252.0)), assets)
}

fn with_budget_constraints(problem: Problem, assets: usize) -> Problem {
    let labels = problem.labels.clone();
    let expected: Vec<f64> = (0..assets)
        .map(|i| 0.02 + 0.06 * ((i * 37 % 101) as f64 / 100.0))
        .collect();
    let groups = (0..10)
        .map(|g| GroupLimit {
            name: format!("group {g}"),
            assets: labels.iter().skip(g).step_by(10).cloned().collect(),
            min: 0.02,
            max: 0.2,
        })
        .collect();
    problem
        .with_expected_returns(expected)
        .with_constraints(Constraints {
            bounds: finalytics::opt::Bound {
                min: 0.0,
                max: 0.05,
            },
            groups,
            ..Default::default()
        })
}
