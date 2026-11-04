//! The rebalance report: the trades that move a portfolio's accounts to target weights, how the
//! weights move, what the trades change and what they cost, and what to check before making them.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Currency, Date, Issue, Result};
use finalytics_ledger::{Account, HoldingKey, LotMethod};
use finalytics_opt::{Fees, RebalanceOptions, RebalanceResults, Side, TaxRates};
use finalytics_risk::{CovarianceMethod, Matrix};

use crate::RiskReturn;
use crate::backtest_html::{
    Frame, SERIES, capitalise, frame, key, limit_in_sentence, lower_name, notes_table, percent,
    section, span, table,
};
use crate::html::{card, esc, ticks, tiles};
use crate::text::{
    Page, WIDTH, covariance_name, date, dec, money, money_places, pad_left, pad_right, pct, plural,
    width, wrap,
};

/// The trades that move a portfolio's accounts to target weights: how each holding's weight
/// moves, what the trades change and cost, each account's trades, and what to check before
/// making them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RebalanceReport {
    /// What was rebalanced, to what, and how.
    pub context: RebalanceContext,
    /// The trades and their costs, and what they change.
    pub summary: RebalanceSummary,
    /// Each holding's weight before the trades, at the target and after, in the accounts
    /// together: the largest target first, then cash.
    pub holdings: Vec<HoldingMove>,
    /// Each account's trades, conversions, costs and warnings.
    pub accounts: Vec<AccountTrades>,
    /// The report's title, such as "Morgan family portfolio": shown when it is printed or laid
    /// out as a page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// What a rebalance report describes: the date, the targets, the accounts and the choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RebalanceContext {
    /// The engine's version.
    pub version: String,
    /// A SHA-256 digest of the targets, the choices and the proposal.
    pub digest: String,
    /// The date of the prices.
    pub date: Date,
    /// The currency of the values.
    pub base_currency: Currency,
    /// The method of the optimisation the targets came from, such as "Minimum variance"; absent
    /// for weights given by holding.
    pub method: Option<String>,
    /// That optimisation's limits in words, such as "5% to 40% in each asset".
    pub limits: Vec<String>,
    /// Whether trades are in whole units.
    pub whole_units: bool,
    /// How sales choose lots when no tax rates are given: absent for each account's own
    /// method.
    pub lot_method: Option<LotMethod>,
    /// The smallest trade, in the base currency.
    pub minimum_trade: f64,
    /// The fees each trade pays.
    pub fees: Fees,
    /// The tax rates on realised gains, if given.
    pub tax: Option<TaxRates>,
    /// The estimates behind the volatility and expected return before and after; absent when
    /// the holdings' returns could not be estimated.
    pub estimates: Option<RebalanceEstimates>,
}

/// The estimates behind a rebalance's volatility and expected return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RebalanceEstimates {
    /// The covariance estimator.
    pub covariance: CovarianceMethod,
    /// Return periods in a year.
    pub periods_per_year: f64,
    /// The first date of the returns.
    pub start: Date,
    /// The last date of the returns.
    pub end: Date,
}

/// The trades of a rebalance in figures: how many, how much is traded, what they cost and what
/// they change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RebalanceSummary {
    /// The accounts' combined value.
    pub value_base: f64,
    /// The number of trades.
    pub trades: usize,
    /// The number of sales.
    pub sales: usize,
    /// The number of purchases.
    pub purchases: usize,
    /// The value traded as a share of the combined value.
    pub turnover: f64,
    /// The value traded.
    pub traded_base: f64,
    /// The fees of all the trades.
    pub fees_base: f64,
    /// The gain the sales realise, after fees.
    pub realised_gain_base: f64,
    /// With tax rates, the part of the realised gain from lots held long-term.
    pub long_term_gain_base: Option<f64>,
    /// With tax rates, the estimated tax on the sales in taxable accounts.
    pub tax_base: Option<f64>,
    /// The unrealised gain of the accounts before the trades.
    pub unrealised_gain_base: f64,
    /// Volatility and expected return a year before the trades.
    pub before: Option<RiskReturn>,
    /// After the trades.
    pub after: Option<RiskReturn>,
    /// At the targets.
    pub target: Option<RiskReturn>,
}

/// A holding's weight before the trades, at the target and after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct HoldingMove {
    /// The holding: an instrument's identifier, or a currency followed by ` cash`.
    pub holding: String,
    /// The weight before the trades.
    pub before: f64,
    /// The target weight; base-currency cash takes what the targets leave.
    pub target: f64,
    /// The weight after the trades.
    pub after: f64,
    /// The value after the trades.
    pub value_after_base: f64,
}

/// One account's part of a rebalance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AccountTrades {
    /// The account's identifier.
    pub account: String,
    /// The account's name, or its identifier.
    pub name: String,
    /// Whether its gains are taxed when realised.
    pub taxable: bool,
    /// Its value before the trades.
    pub value_base: f64,
    /// The value traded as a share of its value.
    pub turnover: f64,
    /// Sales, then purchases.
    pub trades: Vec<TradeRow>,
    /// Currency conversions.
    pub conversions: Vec<ConversionRow>,
    /// The fees of its trades.
    pub fees_base: f64,
    /// The gain its sales realise, after fees.
    pub realised_gain_base: f64,
    /// With tax rates, the part of that gain from lots held long-term.
    pub long_term_gain_base: Option<f64>,
    /// With tax rates and a taxable account, the estimated tax on its sales.
    pub tax_base: Option<f64>,
    /// What to check before making the trades: carried prices, a shortfall of cash, and trades
    /// left out.
    pub warnings: Vec<Issue>,
}

/// One proposed trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TradeRow {
    /// Buy or sell.
    pub side: Side,
    /// The units traded.
    pub quantity: f64,
    /// The instrument's identifier.
    pub instrument: String,
    /// The price per unit, in the instrument's currency.
    pub price: f64,
    /// The instrument's currency.
    pub currency: Currency,
    /// The value traded, in the base currency.
    pub value_base: f64,
    /// The fee, in the instrument's currency.
    pub fees_local: f64,
    /// For a sale, the gain it realises, after fees.
    pub realised_gain_base: Option<f64>,
    /// For a sale in a taxable account with tax rates, the estimated tax.
    pub tax_base: Option<f64>,
}

/// One currency conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ConversionRow {
    /// The amount sold.
    pub sell: f64,
    /// Its currency.
    pub sell_currency: Currency,
    /// The amount bought.
    pub buy: f64,
    /// Its currency.
    pub buy_currency: Currency,
}

/// The returns behind the volatility and expected return of a rebalance: annual figures for the
/// instruments, from `start` to `end`.
#[doc(hidden)]
pub struct RebalanceEstimateInputs<'a> {
    pub covariance: &'a Matrix,
    pub expected_returns: &'a [f64],
    pub covariance_method: CovarianceMethod,
    pub periods_per_year: f64,
    pub start: Date,
    pub end: Date,
}

/// What the rebalance report is built from. The `finalytics` crate assembles it.
#[doc(hidden)]
pub struct RebalanceInputs<'a> {
    pub results: &'a RebalanceResults,
    pub targets: &'a BTreeMap<HoldingKey, f64>,
    pub method: Option<String>,
    pub limits: Vec<String>,
    pub accounts: &'a [Account],
    pub options: &'a RebalanceOptions,
    pub unrealised_gain_base: f64,
    pub estimates: Option<RebalanceEstimateInputs<'a>>,
}

/// The rebalance report from what the `finalytics` crate assembles.
#[doc(hidden)]
pub fn rebalance_report(inputs: RebalanceInputs<'_>) -> Result<RebalanceReport> {
    let r = inputs.results;
    let base = r.base_currency;
    let cash = HoldingKey::Cash(base);
    let target_of = |key: &HoldingKey| -> f64 {
        if *key == cash {
            (1.0 - inputs
                .targets
                .iter()
                .filter(|(k, _)| **k != cash)
                .map(|(_, w)| w)
                .sum::<f64>())
            .max(0.0)
        } else {
            inputs.targets.get(key).copied().unwrap_or(0.0)
        }
    };
    let mut keys: Vec<HoldingKey> = r
        .weights_before
        .keys()
        .chain(r.weights_after.keys())
        .chain(inputs.targets.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    keys.sort_by(|a, b| {
        let is_cash = |k: &HoldingKey| matches!(k, HoldingKey::Cash(_));
        is_cash(a)
            .cmp(&is_cash(b))
            .then(target_of(b).total_cmp(&target_of(a)))
            .then(a.cmp(b))
    });
    let value = dec(r.value_base);
    // The weights after trading are of the value left once the fees are paid.
    let value_after = value - dec(r.fees_base);
    let holdings: Vec<HoldingMove> = keys
        .iter()
        .map(|k| {
            let after = r.weights_after.get(k).copied().unwrap_or(0.0);
            HoldingMove {
                holding: k.to_string(),
                before: r.weights_before.get(k).copied().unwrap_or(0.0),
                target: target_of(k),
                after,
                value_after_base: after * value_after,
            }
        })
        .collect();

    // Volatility and expected return of weights by instrument; cash is riskless and earns
    // nothing.
    let estimates = inputs.estimates.as_ref();
    let risk_of = |weights: &dyn Fn(&HoldingKey) -> f64| -> Option<RiskReturn> {
        let e = estimates?;
        let labels = e.covariance.labels();
        let w: Vec<f64> = labels
            .iter()
            .map(|l| l.parse::<HoldingKey>().map_or(0.0, |k| weights(&k)))
            .collect();
        let n = w.len();
        let variance: f64 = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| w[i] * e.covariance.get(i, j) * w[j])
                    .sum::<f64>()
            })
            .sum();
        Some(RiskReturn {
            volatility: variance.max(0.0).sqrt(),
            expected_return: w.iter().zip(e.expected_returns).map(|(a, b)| a * b).sum(),
        })
    };
    let names: BTreeMap<&str, &Account> =
        inputs.accounts.iter().map(|a| (a.id.as_str(), a)).collect();
    let accounts: Vec<AccountTrades> = r
        .accounts
        .iter()
        .map(|a| {
            let account = names.get(a.account.as_str());
            let long_term = inputs.options.tax.as_ref().map(|_| {
                a.trades
                    .iter()
                    .filter_map(|t| t.long_term_gain_base)
                    .map(dec)
                    .sum()
            });
            AccountTrades {
                account: a.account.to_string(),
                name: account
                    .and_then(|x| x.name.clone())
                    .unwrap_or_else(|| a.account.to_string()),
                taxable: a.taxable,
                value_base: dec(a.value_base),
                turnover: a.turnover,
                trades: a
                    .trades
                    .iter()
                    .map(|t| TradeRow {
                        side: t.side,
                        quantity: dec(t.quantity),
                        instrument: t.instrument.to_string(),
                        price: dec(t.price),
                        currency: t.currency,
                        value_base: dec(t.value_base),
                        fees_local: dec(t.fees_local),
                        realised_gain_base: t.realised_gain_base.map(dec),
                        tax_base: t.tax_base.map(dec),
                    })
                    .collect(),
                conversions: a
                    .conversions
                    .iter()
                    .map(|c| ConversionRow {
                        sell: dec(c.sell.amount()),
                        sell_currency: c.sell.currency(),
                        buy: dec(c.buy.amount()),
                        buy_currency: c.buy.currency(),
                    })
                    .collect(),
                fees_base: dec(a.fees_base),
                realised_gain_base: dec(a.realised_gain_base),
                long_term_gain_base: long_term,
                tax_base: a.tax_base.map(dec),
                warnings: a.warnings.issues().to_vec(),
            }
        })
        .collect();
    let trades: Vec<&TradeRow> = accounts.iter().flat_map(|a| &a.trades).collect();
    let sales = trades.iter().filter(|t| t.side == Side::Sell).count();
    let summary = RebalanceSummary {
        value_base: value,
        trades: trades.len(),
        sales,
        purchases: trades.len() - sales,
        turnover: r.turnover,
        traded_base: trades.iter().map(|t| t.value_base).sum(),
        fees_base: dec(r.fees_base),
        realised_gain_base: dec(r.realised_gain_base),
        long_term_gain_base: inputs
            .options
            .tax
            .as_ref()
            .map(|_| accounts.iter().filter_map(|a| a.long_term_gain_base).sum()),
        tax_base: r.tax_base.map(dec),
        unrealised_gain_base: inputs.unrealised_gain_base,
        before: risk_of(&|k| r.weights_before.get(k).copied().unwrap_or(0.0)),
        after: risk_of(&|k| r.weights_after.get(k).copied().unwrap_or(0.0)),
        target: risk_of(&|k| inputs.targets.get(k).copied().unwrap_or(0.0)),
    };
    let mut hash = Sha256::new();
    for (k, w) in inputs.targets {
        hash.update(k.to_string().as_bytes());
        hash.update(w.to_le_bytes());
    }
    hash.update(
        serde_json::to_string(inputs.options)
            .unwrap_or_default()
            .as_bytes(),
    );
    hash.update(serde_json::to_string(r).unwrap_or_default().as_bytes());
    let digest = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let Some(date) = r.accounts.first().map(|a| a.date) else {
        let mut report = finalytics_core::ValidationReport::new();
        report.error("results", "has no accounts");
        return Err(finalytics_core::Error::Validation(report));
    };
    Ok(RebalanceReport {
        context: RebalanceContext {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            digest,
            date,
            base_currency: base,
            method: inputs.method,
            limits: inputs.limits,
            whole_units: inputs.options.whole_units,
            lot_method: inputs.options.lot_method,
            minimum_trade: dec(inputs.options.minimum_trade),
            fees: inputs.options.fees.clone(),
            tax: inputs.options.tax.clone(),
            estimates: estimates.map(|e| RebalanceEstimates {
                covariance: e.covariance_method,
                periods_per_year: e.periods_per_year,
                start: e.start,
                end: e.end,
            }),
        },
        summary,
        holdings,
        accounts,
        name: None,
    })
}

// ── Words ────────────────────────────────────────────────────────────────────────────────────

/// An amount in the base currency, whole: "$696,578" for US dollars, "696,578 EUR" otherwise.
fn base_money(v: f64, currency: Currency) -> String {
    let symbol = match currency.code() {
        "USD" => Some("$"),
        "GBP" => Some("£"),
        "EUR" => Some("€"),
        _ => None,
    };
    match symbol {
        Some(s) if v < 0.0 => format!("−{s}{}", money(-v)),
        Some(s) => format!("{s}{}", money(v)),
        None => format!("{} {currency}", money(v)),
    }
}

/// An amount with two decimals and its currency.
fn amount(v: f64, currency: Currency) -> String {
    format!("{} {currency}", money_places(v, 2))
}

/// Units: whole numbers without decimals.
fn units(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        money(v)
    } else {
        money_places(v, 4)
    }
}

fn signed(v: f64) -> String {
    pct(Some(if v.abs() < 5e-5 { 0.0 } else { v }), 1, true)
}

fn side(s: Side) -> &'static str {
    match s {
        Side::Buy => "Buy",
        Side::Sell => "Sell",
    }
}

impl RebalanceReport {
    fn money(&self, v: f64) -> String {
        base_money(v, self.context.base_currency)
    }

    /// "To minimum variance · 5% to 40% in each asset · whole units · tax-aware".
    fn headline(&self) -> String {
        let c = &self.context;
        let mut parts = vec![match &c.method {
            Some(m) => format!("To {}", lower_name(m)),
            None => "To the target weights".to_owned(),
        }];
        parts.extend(c.limits.iter().map(|l| limit_in_sentence(l)));
        parts.push(
            if c.whole_units {
                "whole units"
            } else {
                "fractional units"
            }
            .to_owned(),
        );
        if c.tax.is_some() {
            parts.push("tax-aware".to_owned());
        }
        parts.join(" · ")
    }

    fn untaxed(&self) -> Vec<String> {
        self.accounts
            .iter()
            .filter(|a| !a.taxable)
            .map(|a| a.name.clone())
            .collect()
    }

    fn method_rows(&self) -> Vec<(&'static str, String)> {
        let c = &self.context;
        let names: Vec<String> = self.accounts.iter().map(|a| a.name.clone()).collect();
        let mut rows = vec![(
            "Targets",
            match &c.method {
                Some(m) if c.limits.is_empty() => format!("{m}, from an optimisation."),
                Some(m) => format!(
                    "{m}, {}, from an optimisation.",
                    c.limits
                        .iter()
                        .map(|l| limit_in_sentence(l))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                None => "Weights given by holding.".to_owned(),
            },
        )];
        rows.push((
            "Accounts",
            if names.len() > 1 {
                format!(
                    "{}, rebalanced together. A holding above its target is sold from the \
                     accounts that hold it, in proportion to their holdings; one below it is \
                     bought in proportion to the value each account has free. Trades do not move \
                     cash between accounts.",
                    crate::backtest_html::and_list(&names)
                )
            } else {
                format!("{}.", names.join(""))
            },
        ));
        rows.push((
            "Prices",
            format!(
                "{}; other currencies at the exchange rates of that day.",
                date(c.date)
            ),
        ));
        let mut trades = if c.whole_units {
            "Whole units, each rounded towards zero.".to_owned()
        } else {
            "Fractional units.".to_owned()
        };
        if c.minimum_trade > 0.0 {
            let _ = write!(
                trades,
                " Trades worth less than {} are left out.",
                self.money(c.minimum_trade)
            );
        }
        trades.push_str(
            " Purchases leave room for the fees, so the trades fit the cash each account has.",
        );
        rows.push(("Trades", trades));
        rows.push((
            "Lots",
            if c.tax.is_some() {
                "In taxable accounts, each sale takes the lots that cost the least tax, losses \
                 first."
                    .to_owned()
            } else {
                match c.lot_method {
                    Some(m) => format!("{}.", capitalise(&lot_words(m))),
                    None => "Each account's own lot method (first in, first out unless the \
                             account says otherwise)."
                        .to_owned(),
                }
            },
        ));
        rows.push(("Fees", fee_words(&c.fees)));
        rows.push((
            "Tax",
            match &c.tax {
                Some(t) => {
                    let mut text = format!(
                        "{} on short-term and {} on long-term gains (held more than {} days), \
                         after fees",
                        pct(Some(dec(t.short_term)), 0, false),
                        pct(Some(dec(t.long_term)), 0, false),
                        t.long_term_after_days
                    );
                    let untaxed = self.untaxed();
                    if !untaxed.is_empty() {
                        let _ = write!(
                            text,
                            "; {} not taxable",
                            crate::backtest_html::and_list(&untaxed)
                        );
                    }
                    text.push('.');
                    text
                }
                None => "Not estimated: no tax rates were given.".to_owned(),
            },
        ));
        if let Some(e) = &c.estimates {
            rows.push((
                "Estimates",
                format!(
                    "Volatility and expected return from the holdings' {} returns, {}, \
                     annualised: {} and historical means. Cash is riskless and \
                     earns nothing here.",
                    crate::backtest_html::frequency(e.periods_per_year),
                    span(e.start, e.end),
                    covariance_name(&e.covariance)
                ),
            ));
        }
        rows
    }
}

fn lot_words(method: LotMethod) -> String {
    match method {
        LotMethod::Fifo => "first in, first out",
        LotMethod::Lifo => "last in, first out",
        LotMethod::HighestCost => "the highest-cost lots first",
        LotMethod::LowestCost => "the lowest-cost lots first",
        LotMethod::AverageCost => "average cost",
        LotMethod::SpecificId => "the lots each sale names",
        _ => "the account's lot method",
    }
    .to_owned()
}

fn fee_words(fees: &Fees) -> String {
    let (rate, per_trade, minimum) = (dec(fees.rate), dec(fees.per_trade), dec(fees.minimum));
    if rate == 0.0 && per_trade == 0.0 && minimum == 0.0 {
        return "None.".to_owned();
    }
    let mut parts = Vec::new();
    if rate > 0.0 {
        parts.push(format!("{} of the value traded", pct(Some(rate), 2, false)));
    }
    if per_trade > 0.0 {
        parts.push(format!("{} a trade", money_places(per_trade, 2)));
    }
    let mut text = parts.join(" plus ");
    if minimum > 0.0 {
        let _ = write!(text, ", at least {}", money_places(minimum, 2));
    }
    format!("{}, in the instrument's currency.", capitalise(&text))
}

// ── The page ─────────────────────────────────────────────────────────────────────────────────

impl RebalanceReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own, or
    /// "Rebalance"): what the trades change and cost, how each holding's weight moves, each
    /// account's trades, the costs and tax, what to check, and the method.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let s = &self.summary;
        let title = name.or(self.name.as_deref()).unwrap_or("Rebalance");
        let instruments: Vec<&HoldingMove> = self
            .holdings
            .iter()
            .filter(|h| !h.holding.ends_with(" cash"))
            .collect();
        let colour = |holding: &str| -> &'static str {
            instruments
                .iter()
                .position(|h| h.holding == holding)
                .map_or("var(--muted-fill)", |k| SERIES[k % SERIES.len()])
        };

        // Summary.
        let mut items = vec![
            (
                "Trades".to_owned(),
                s.trades.to_string(),
                Some(format!(
                    "{} and {} in {}",
                    plural(s.sales, "sale"),
                    plural(s.purchases, "purchase"),
                    plural(self.accounts.len(), "account")
                )),
            ),
            (
                "Turnover".to_owned(),
                percent(s.turnover),
                Some(format!(
                    "One-way: half the {} traded, of {}",
                    self.money(s.traded_base),
                    self.money(s.value_base)
                )),
            ),
        ];
        if let (Some(b), Some(a)) = (&s.before, &s.after) {
            items.push((
                "Volatility".to_owned(),
                format!("{} → {}", percent(b.volatility), percent(a.volatility)),
                s.target
                    .as_ref()
                    .map(|t| format!("Target {}", percent(t.volatility))),
            ));
            items.push((
                "Expected return".to_owned(),
                format!(
                    "{} → {}",
                    percent(b.expected_return),
                    percent(a.expected_return)
                ),
                s.target
                    .as_ref()
                    .map(|t| format!("Target {}", percent(t.expected_return))),
            ));
        }
        items.push(match s.tax_base {
            Some(tax) => (
                "Fees and tax".to_owned(),
                // The total of the two amounts as shown, so that they add up.
                self.money(crate::text::round(s.fees_base, 0) + crate::text::round(tax, 0)),
                Some(format!(
                    "Fees {}, tax {}",
                    self.money(s.fees_base),
                    self.money(tax)
                )),
            ),
            None => (
                "Fees".to_owned(),
                self.money(s.fees_base),
                Some(format!(
                    "Realised gains {}",
                    self.money(s.realised_gain_base)
                )),
            ),
        });
        let mut body = tiles(&items);
        body.push_str(&card(
            "Where the weights move",
            &(self.moves_chart(&colour) + &self.moves_table(&colour)),
            Some(&self.moves_note()),
        ));
        let mut html = section(
            "r-summary",
            "Summary",
            "What the trades change, and what they cost",
            &body,
        );

        // Trades.
        let mut trades = String::new();
        let taxed = c.tax.is_some();
        for a in &self.accounts {
            let mut rows: Vec<Vec<String>> = a
                .trades
                .iter()
                .map(|t| {
                    let mut row = vec![
                        side(t.side).to_owned(),
                        units(t.quantity),
                        esc(&t.instrument),
                        esc(&amount(t.price, t.currency)),
                        esc(&self.money(t.value_base)),
                        esc(&amount(t.fees_local, t.currency)),
                        t.realised_gain_base
                            .map_or_else(String::new, |g| esc(&self.money(g))),
                    ];
                    if taxed {
                        row.push(match (t.side, t.tax_base) {
                            (_, Some(tax)) => esc(&self.money(tax)),
                            (Side::Sell, None) => "Not taxed".to_owned(),
                            _ => String::new(),
                        });
                    }
                    row
                })
                .collect();
            for v in &a.conversions {
                let mut row = vec![
                    "Convert".to_owned(),
                    String::new(),
                    format!("{} to {}", v.sell_currency, v.buy_currency),
                    format!("{:.5} {}", v.sell / v.buy, v.sell_currency),
                    if v.sell_currency == c.base_currency {
                        esc(&self.money(v.sell))
                    } else {
                        esc(&self.money(v.buy))
                    },
                    String::new(),
                    String::new(),
                ];
                if taxed {
                    row.push(String::new());
                }
                rows.push(row);
            }
            let mut head = vec![
                "Side",
                "Units",
                "Instrument",
                "Price",
                "Value",
                "Fee",
                "Gain",
            ];
            let mut right = vec![false, true, false, true, true, true, true];
            if taxed {
                head.push("Tax");
                right.push(true);
            }
            let mut note = format!(
                "Sales first, then purchases. {} is worth {}; the trades turn over {} of it.",
                a.name,
                self.money(a.value_base),
                percent(a.turnover)
            );
            for v in &a.conversions {
                let _ = write!(
                    note,
                    " The conversion sells {} for {}.",
                    amount(v.sell, v.sell_currency),
                    amount(v.buy, v.buy_currency)
                );
            }
            if !a.taxable {
                note.push_str(" Not a taxable account: its sales are not taxed.");
            }
            trades.push_str(&card(&a.name, &aligned(&head, &rows, &right), Some(&note)));
        }
        html.push_str(&section(
            "r-trades",
            "Trades",
            "Each account's trades, sales first",
            &trades,
        ));

        // Costs.
        let mut head: Vec<String> = vec!["Account".into(), "Fees".into(), "Realised gain".into()];
        if taxed {
            head.extend([
                "Long-term".into(),
                "Short-term".into(),
                "Estimated tax".into(),
            ]);
        }
        let mut rows: Vec<Vec<String>> = Vec::new();
        for a in &self.accounts {
            let mut row = vec![
                esc(&a.name),
                esc(&self.money(a.fees_base)),
                esc(&self.money(a.realised_gain_base)),
            ];
            if taxed {
                let long = a.long_term_gain_base.unwrap_or(0.0);
                row.push(esc(&self.money(long)));
                row.push(esc(&self.money(a.realised_gain_base - long)));
                row.push(match a.tax_base {
                    Some(t) if a.taxable => esc(&self.money(t)),
                    _ => "Not taxed".to_owned(),
                });
            }
            rows.push(row);
        }
        if self.accounts.len() > 1 {
            let mut row = vec![
                "<b>Total</b>".to_owned(),
                format!("<b>{}</b>", esc(&self.money(s.fees_base))),
                format!("<b>{}</b>", esc(&self.money(s.realised_gain_base))),
            ];
            if taxed {
                let long = s.long_term_gain_base.unwrap_or(0.0);
                row.push(format!("<b>{}</b>", esc(&self.money(long))));
                row.push(format!(
                    "<b>{}</b>",
                    esc(&self.money(s.realised_gain_base - long))
                ));
                row.push(format!(
                    "<b>{}</b>",
                    esc(&self.money(s.tax_base.unwrap_or(0.0)))
                ));
            }
            rows.push(row);
        }
        let mut note = format!("Fees: {}", lower_first_word(&fee_words(&c.fees)));
        if let Some(t) = &c.tax {
            let _ = write!(
                note,
                " Tax is {} on short-term and {} on long-term gains (held more than {} days), \
                 after fees; in taxable accounts each sale takes the lots that cost the least tax.",
                pct(Some(dec(t.short_term)), 0, false),
                pct(Some(dec(t.long_term)), 0, false),
                t.long_term_after_days
            );
        }
        if s.unrealised_gain_base > 0.0 {
            let _ = write!(
                note,
                " The sales realise {} of the unrealised gains of {}.",
                percent(s.realised_gain_base / s.unrealised_gain_base),
                self.money(s.unrealised_gain_base)
            );
        }
        html.push_str(&section(
            "r-costs",
            "Costs",
            "The fees, the gains the sales realise, and the tax on them",
            &card(
                "What the trades cost",
                &table(&head, &rows, 1, "cmp"),
                Some(&note),
            ),
        ));

        // Check.
        let warnings: Vec<(&str, &Issue)> = self
            .accounts
            .iter()
            .flat_map(|a| a.warnings.iter().map(move |w| (a.name.as_str(), w)))
            .collect();
        let mut nav = vec![
            ("r-summary", "Summary"),
            ("r-trades", "Trades"),
            ("r-costs", "Costs"),
        ];
        if !warnings.is_empty() {
            let rows: Vec<Vec<String>> = warnings
                .iter()
                .map(|(n, w)| vec![esc(n), esc(&format!("{}.", capitalise(&issue_words(w))))])
                .collect();
            html.push_str(&section(
                "r-check",
                "Check",
                "What to look at before making the trades",
                &card(
                    "Check before trading",
                    &aligned(&["Account", "What to check"], &rows, &[false, false]),
                    None,
                ),
            ));
            nav.push(("r-check", "Check"));
        }
        html.push_str(&section(
            "r-method",
            "Method",
            "How the trades were found",
            &card(
                "How the trades were found",
                &notes_table(&self.method_rows()),
                None,
            ),
        ));
        nav.push(("r-method", "Method"));
        let meta = format!(
            "{}<br>{} · {} · prices of {}",
            esc(&self.headline()),
            esc(&plural(self.accounts.len(), "account")),
            esc(&self.money(s.value_base)),
            esc(&date(c.date))
        );
        frame(
            &Frame {
                title,
                brand: Some("Rebalance report"),
                meta: &meta,
                nav: &nav,
                version: &c.version,
                digest: &c.digest,
                source: "the portfolio's records and prices",
            },
            &html,
        )
    }

    fn moves_note(&self) -> String {
        let mut note = format!(
            "Weights of {} together, {}, at the prices of {}. After differs from the target by \
             {}the fees; base-currency cash takes what the targets leave.",
            if self.accounts.len() == 1 {
                "the account".to_owned()
            } else {
                format!("the {} accounts", self.accounts.len())
            },
            self.money(self.summary.value_base),
            date(self.context.date),
            if self.context.whole_units {
                "whole units and "
            } else {
                ""
            }
        );
        if self.summary.before.is_some() {
            note.push_str(
                " Volatility and expected return come from the estimates the method describes.",
            );
        }
        note
    }

    /// Each holding's weight before, at the target and after: a line from before to after,
    /// with the target marked.
    fn moves_chart(&self, colour: &dyn Fn(&str) -> &'static str) -> String {
        let (w, row, l, r) = (880.0, 30.0, 110.0, 130.0);
        let top = self
            .holdings
            .iter()
            .map(|h| h.before.max(h.after).max(h.target))
            .fold(0.0, f64::max);
        let gx = ticks(0.0, top.max(0.05), 6);
        let (x0, x1) = (gx[0], gx.last().copied().unwrap_or(1.0));
        let x = |v: f64| l + (w - l - r) * (v - x0) / (x1 - x0).max(1e-12);
        let h = row * self.holdings.len() as f64 + 34.0;
        let mut out = format!(r#"<svg viewBox="0 0 {w} {h}" role="img">"#);
        for v in &gx {
            let _ = write!(
                out,
                r#"<line class="grid" x1="{0:.1}" x2="{0:.1}" y1="4" y2="{1:.1}"/><text class="tick" x="{0:.1}" y="{2:.1}" text-anchor="middle">{3}</text>"#,
                x(*v),
                h - 26.0,
                h - 8.0,
                esc(&pct(Some(*v), 0, false))
            );
        }
        for (k, m) in self.holdings.iter().enumerate() {
            let y = 18.0 + k as f64 * row;
            let c = colour(&m.holding);
            let _ = write!(
                out,
                r#"<text class="lab" x="{:.1}" y="{:.1}" text-anchor="end">{}</text><g><title>{}: {} before, {} target, {} after</title><line x1="{:.1}" x2="{:.1}" y1="{y}" y2="{y}" style="stroke:{c};stroke-width:3;stroke-linecap:round;opacity:.55"/><line x1="{:.1}" x2="{:.1}" y1="{:.1}" y2="{:.1}" style="stroke:var(--ink);stroke-width:2"/><circle cx="{:.1}" cy="{y}" r="5" style="fill:var(--surface);stroke:var(--ink2);stroke-width:2"/><circle cx="{:.1}" cy="{y}" r="6" style="fill:{c};stroke:var(--surface);stroke-width:2"/></g><text class="val" x="{:.1}" y="{:.1}">{} → {}</text>"#,
                l - 14.0,
                y + 4.0,
                esc(&m.holding),
                esc(&m.holding),
                percent(m.before),
                percent(m.target),
                percent(m.after),
                x(m.before),
                x(m.after),
                x(m.target),
                x(m.target),
                y - 9.0,
                y + 9.0,
                x(m.before),
                x(m.after),
                w - r + 14.0,
                y + 4.0,
                percent(m.before),
                percent(m.after)
            );
        }
        out.push_str("</svg>");
        format!(
            r#"<div class="legend"><span class="key"><i class="hollow"></i>Before</span><span class="key"><i style="background:var(--ink);width:3px;border-radius:0"></i>Target</span>{}</div>{out}"#,
            key("After", "var(--ink2)")
        )
    }

    fn moves_table(&self, colour: &dyn Fn(&str) -> &'static str) -> String {
        let rows: Vec<Vec<String>> = self
            .holdings
            .iter()
            .map(|m| {
                vec![
                    key(&m.holding, colour(&m.holding)),
                    percent(m.before),
                    percent(m.target),
                    percent(m.after),
                    signed(m.after - m.before),
                    esc(&self.money(m.value_after_base)),
                ]
            })
            .collect();
        let head: Vec<String> = [
            "Holding",
            "Before",
            "Target",
            "After",
            "Change",
            "Value after",
        ]
        .iter()
        .map(|h| (*h).to_owned())
        .collect();
        table(&head, &rows, 1, "cmp")
    }
}

/// A table whose columns are right-aligned where `right` says; cells are HTML.
fn aligned(head: &[&str], rows: &[Vec<String>], right: &[bool]) -> String {
    let class = |k: usize| {
        if right.get(k).copied().unwrap_or(false) {
            r#" class="r""#
        } else {
            ""
        }
    };
    let mut out = String::from(r#"<div class="tw"><table class="cmp"><thead><tr>"#);
    for (k, h) in head.iter().enumerate() {
        let _ = write!(out, "<th{}>{}</th>", class(k), esc(h));
    }
    out.push_str("</tr></thead><tbody>");
    for row in rows {
        out.push_str("<tr>");
        for (k, c) in row.iter().enumerate() {
            let _ = write!(out, "<td{}>{c}</td>", class(k));
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table></div>");
    out
}

fn lower_first_word(text: &str) -> String {
    crate::backtest_html::lower_first(text)
}

/// A warning in words: the shortfall of cash, or a price or trade left out.
fn issue_words(issue: &Issue) -> String {
    match issue.path.split_once('.') {
        Some(("prices", id)) => format!("{id} {}", issue.message),
        Some(("trades", id)) => format!("{id}: {}", issue.message),
        _ => issue.message.clone(),
    }
}

// ── The text ─────────────────────────────────────────────────────────────────────────────────

impl RebalanceReport {
    /// The report as text for printing, 80 columns wide, titled with `name` (or the report's
    /// own) if given.
    pub fn to_text(&self, name: Option<&str>) -> String {
        let c = &self.context;
        let s = &self.summary;
        let name = name.or(self.name.as_deref());
        let mut page = Page::new(None);
        page.rule('━');
        let left = "FINALYTICS  ·  REBALANCE";
        page.line(format!(
            "{left}{}",
            pad_left(&date(c.date), WIDTH.saturating_sub(width(left)))
        ));
        if let Some(name) = name {
            page.line(name);
        }
        for line in wrap(&self.headline(), WIDTH) {
            page.line(line);
        }
        page.line(format!(
            "{} · {} · {} · one-way turnover {}",
            plural(self.accounts.len(), "account"),
            self.money(s.value_base),
            plural(s.trades, "trade"),
            percent(s.turnover)
        ));
        page.rule('━');

        page.section("Weights");
        let rows: Vec<Vec<String>> = self
            .holdings
            .iter()
            .map(|m| {
                vec![
                    m.holding.clone(),
                    percent(m.before),
                    percent(m.target),
                    percent(m.after),
                    signed(m.after - m.before),
                    self.money(m.value_after_base),
                ]
            })
            .collect();
        page.table(
            &[
                "Holding",
                "Before",
                "Target",
                "After",
                "Change",
                "Value after",
            ],
            &rows,
            None,
        );
        page.blank();
        if let (Some(b), Some(a)) = (&s.before, &s.after) {
            let mut text = format!(
                "Volatility {} → {}",
                percent(b.volatility),
                percent(a.volatility)
            );
            if let Some(t) = &s.target {
                let _ = write!(text, " (target {})", percent(t.volatility));
            }
            let _ = write!(
                text,
                "; expected return {} → {}.",
                percent(b.expected_return),
                percent(a.expected_return)
            );
            page.note(&text);
        }

        page.section("Trades");
        // One layout for every account, so the columns line up.
        let lines: Vec<Vec<[String; 6]>> = self
            .accounts
            .iter()
            .map(|a| {
                a.trades
                    .iter()
                    .map(|t| {
                        let mut extra = String::new();
                        if let Some(g) = t.realised_gain_base {
                            extra = format!("gain {}", self.money(g));
                            match t.tax_base {
                                Some(tax) => {
                                    let _ = write!(extra, ", tax {}", self.money(tax));
                                }
                                None if c.tax.is_some() && !a.taxable => {
                                    extra.push_str(", not taxed");
                                }
                                None => {}
                            }
                        }
                        [
                            side(t.side).to_owned(),
                            units(t.quantity),
                            t.instrument.clone(),
                            amount(t.price, t.currency),
                            self.money(t.value_base),
                            extra,
                        ]
                    })
                    .collect()
            })
            .collect();
        let widths: Vec<usize> = (0..5)
            .map(|k| {
                lines
                    .iter()
                    .flatten()
                    .map(|row| width(&row[k]))
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for (k, (a, rows)) in self.accounts.iter().zip(&lines).enumerate() {
            if k > 0 {
                page.blank();
            }
            page.line(format!(
                "  {} · {} · one-way turnover {}",
                a.name,
                self.money(a.value_base),
                percent(a.turnover)
            ));
            for row in rows {
                let text = format!(
                    "    {}  {}  {}  {}  {}",
                    pad_right(&row[0], widths[0]),
                    pad_left(&row[1], widths[1]),
                    pad_right(&row[2], widths[2]),
                    pad_left(&row[3], widths[3]),
                    pad_left(&row[4], widths[4])
                );
                if row[5].is_empty() {
                    page.line(text.trim_end());
                } else if width(&text) + 3 + width(&row[5]) <= WIDTH {
                    page.line(format!("{text}   {}", row[5]));
                } else {
                    page.line(text.trim_end());
                    page.line(format!("      {}", row[5]));
                }
            }
            for v in &a.conversions {
                page.line(format!(
                    "    Convert {} to {}",
                    amount(v.sell, v.sell_currency),
                    amount(v.buy, v.buy_currency)
                ));
            }
        }

        page.section(if c.tax.is_some() {
            "Costs and tax"
        } else {
            "Costs"
        });
        let mut head = vec!["", "Fees", "Gain"];
        if c.tax.is_some() {
            head.extend(["Long-term", "Short-term", "Tax"]);
        }
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut push = |label: &str, fees: f64, gain: f64, long: Option<f64>, tax: String| {
            let mut row = vec![label.to_owned(), self.money(fees), self.money(gain)];
            if c.tax.is_some() {
                let long = long.unwrap_or(0.0);
                row.push(self.money(long));
                row.push(self.money(gain - long));
                row.push(tax);
            }
            rows.push(row);
        };
        for a in &self.accounts {
            push(
                &a.name,
                a.fees_base,
                a.realised_gain_base,
                a.long_term_gain_base,
                match a.tax_base {
                    Some(t) if a.taxable => self.money(t),
                    _ => "not taxed".to_owned(),
                },
            );
        }
        if self.accounts.len() > 1 {
            push(
                "Total",
                s.fees_base,
                s.realised_gain_base,
                s.long_term_gain_base,
                self.money(s.tax_base.unwrap_or(0.0)),
            );
        }
        page.table(&head, &rows, None);
        page.blank();
        let mut note = format!("Fees: {}", lower_first_word(&fee_words(&c.fees)));
        if let Some(t) = &c.tax {
            let _ = write!(
                note,
                " Tax {} short-term and {} long-term, after fees.",
                pct(Some(dec(t.short_term)), 0, false),
                pct(Some(dec(t.long_term)), 0, false)
            );
        }
        page.note(&note);

        let warnings: Vec<(&str, &Issue)> = self
            .accounts
            .iter()
            .flat_map(|a| a.warnings.iter().map(move |w| (a.name.as_str(), w)))
            .collect();
        if !warnings.is_empty() {
            page.section("Check before trading");
            for (n, w) in warnings {
                page.note(&format!("{n}: {}.", issue_words(w)));
            }
        }

        page.section("How the trades were found");
        let rows = self.method_rows();
        let lw = rows.iter().map(|(l, _)| width(l)).max().unwrap_or(0);
        for (label, text) in &rows {
            for (k, line) in wrap(text, WIDTH - 4 - lw).into_iter().enumerate() {
                let label = if k == 0 { *label } else { "" };
                page.line(format!("  {}  {line}", pad_right(label, lw)));
            }
        }
        page.blank();
        page.line(format!(
            "  Finalytics {} · inputs {}",
            c.version,
            c.digest.chars().take(12).collect::<String>()
        ));
        page.finish()
    }
}

impl fmt::Display for RebalanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text(None))
    }
}
