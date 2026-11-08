//! The portfolio document: a whole portfolio, and optionally its market data, as JSON.

use std::collections::BTreeMap;

use finalytics_core::{Currency, Decimal, Error, Result, TimeSeries, ValidationReport};
use finalytics_ledger::{InstrumentId, MarketData, PortfolioData};
use finalytics_plan::PlanOptions;
use serde::{Deserialize, Serialize};

/// The value of the document's `format` field.
pub const FORMAT: &str = "finalytics.portfolio";

/// The version of the document format this release writes, and the newest it reads.
pub const FORMAT_VERSION: u32 = 1;

/// A portfolio and, optionally, the market data to value it, in one versioned document.
///
/// This is how applications save and load work, and how the Rust, Python and JavaScript
/// interfaces exchange portfolios.
///
/// ```json
/// {
///   "format": "finalytics.portfolio",
///   "format_version": 1,
///   "portfolio": { "base_currency": "USD", "accounts": [], "instruments": [], "transactions": [] },
///   "market": { "prices": { "AAPL": { "dates": ["2026-11-02"], "values": ["201.50"] } } }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PortfolioDocument {
    /// The portfolio.
    pub portfolio: PortfolioData,
    /// Market data stored with the portfolio, if any.
    pub market: Option<MarketData>,
    /// Goals kept with the portfolio, each a plan with its name; none by default.
    pub goals: Vec<PlanOptions>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct DocumentRepr {
    /// Always `"finalytics.portfolio"`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = FORMAT)))]
    format: String,
    /// The version of the format, 1 for this release.
    #[cfg_attr(feature = "schema", schemars(extend("const" = FORMAT_VERSION)))]
    format_version: u32,
    /// The portfolio.
    portfolio: PortfolioData,
    /// Market data stored with the portfolio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    market: Option<MarketRepr>,
    /// Goals kept with the portfolio, each a plan with its name, in the order given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    goals: Vec<PlanOptions>,
}

#[derive(Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "DocumentMarketData"))]
#[serde(deny_unknown_fields)]
/// Market data stored in a portfolio document.
struct MarketRepr {
    /// The currency through which FX rates are crossed; USD by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pivot: Option<Currency>,
    /// How many days a price or rate may be carried forward; 5 by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_carry_days: Option<u32>,
    /// Prices by instrument, in each instrument's currency.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    prices: BTreeMap<InstrumentId, TimeSeries<Decimal>>,
    /// FX rates: units of `quote` per unit of `base`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fx: Vec<FxRepr>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "DocumentFxRates"))]
#[serde(deny_unknown_fields)]
/// The rates for one currency pair in a portfolio document.
struct FxRepr {
    /// The currency priced.
    base: Currency,
    /// The currency it is priced in.
    quote: Currency,
    /// Units of `quote` per unit of `base`.
    rates: TimeSeries<Decimal>,
}

impl MarketRepr {
    fn from_market(m: &MarketData) -> MarketRepr {
        MarketRepr {
            pivot: Some(m.pivot()),
            max_carry_days: Some(m.max_carry_days()),
            prices: m
                .all_prices()
                .map(|(id, s)| (id.clone(), s.clone()))
                .collect(),
            fx: m
                .all_fx()
                .map(|((base, quote), rates)| FxRepr {
                    base,
                    quote,
                    rates: rates.clone(),
                })
                .collect(),
        }
    }

    fn into_market(self) -> MarketData {
        let mut market = MarketData::new();
        if let Some(pivot) = self.pivot {
            market = market.with_pivot(pivot);
        }
        if let Some(days) = self.max_carry_days {
            market = market.with_max_carry_days(days);
        }
        for (id, prices) in self.prices {
            market = market.with_prices(id, prices);
        }
        for fx in self.fx {
            market = market.with_fx(fx.base, fx.quote, fx.rates);
        }
        market
    }
}

/// Market data in the portfolio document's format (the document's `market` member), for
/// reading and writing market data on its own.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; pub mod formats { pub use finalytics_formats::*; } }
/// use finalytics::formats::MarketDocument;
///
/// let text = r#"{"prices": {"AAPL": {"dates": ["2026-11-02"], "values": ["201.50"]}}}"#;
/// let MarketDocument(market) = serde_json::from_str(text)?;
/// assert_eq!(market.pivot().code(), "USD");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarketDocument(pub MarketData);

impl Serialize for MarketDocument {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        MarketRepr::from_market(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MarketDocument {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        MarketRepr::deserialize(deserializer).map(|m| MarketDocument(m.into_market()))
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for MarketDocument {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DocumentMarketData".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        generator.subschema_for::<MarketRepr>()
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for PortfolioDocument {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PortfolioDocument".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = DocumentRepr::json_schema(generator);
        schema.insert(
            "description".into(),
            "A portfolio and, optionally, the market data to value it, in one versioned document."
                .into(),
        );
        schema
    }
}

impl PortfolioDocument {
    /// A document holding `portfolio` without market data.
    pub fn new(portfolio: PortfolioData) -> PortfolioDocument {
        PortfolioDocument {
            portfolio,
            market: None,
            goals: Vec::new(),
        }
    }

    /// Writes the document as indented JSON.
    pub fn to_json(&self) -> String {
        let market = self.market.as_ref().map(MarketRepr::from_market);
        let repr = DocumentRepr {
            format: FORMAT.to_owned(),
            format_version: FORMAT_VERSION,
            portfolio: self.portfolio.clone(),
            market,
            goals: self.goals.clone(),
        };
        // Every field serialises to JSON without error: map keys are strings and no float is
        // non-finite.
        serde_json::to_string_pretty(&repr).unwrap_or_default()
    }

    /// Reads a document, reporting where it is malformed (for example
    /// `portfolio.transactions[3].quantity`).
    ///
    /// The portfolio's structure is validated as well ([`PortfolioData::validate`]); its problems
    /// are reported with paths under `portfolio`.
    pub fn from_json(text: &str) -> Result<PortfolioDocument> {
        // A well-formed document of this version reads directly; anything else is read again,
        // more slowly, to report what is wrong and where.
        if let Ok(repr) = serde_json::from_str::<DocumentRepr>(text)
            && repr.format == FORMAT
            && repr.format_version == FORMAT_VERSION
        {
            return Self::from_repr(repr, ValidationReport::new());
        }
        let mut report = ValidationReport::new();
        let value: serde_json::Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(e) => {
                report.error("", format!("not valid JSON: {e}"));
                return Err(Error::Validation(report));
            }
        };
        match value.get("format").and_then(|f| f.as_str()) {
            Some(FORMAT) => {}
            Some(other) => report.error("format", format!("expected {FORMAT:?}, found {other:?}")),
            None => report.error("format", format!("missing; expected {FORMAT:?}")),
        }
        match value.get("format_version").and_then(|v| v.as_u64()) {
            Some(v) if v == u64::from(FORMAT_VERSION) => {}
            Some(v) if v > u64::from(FORMAT_VERSION) => report.error(
                "format_version",
                format!(
                    "this document uses format version {v}, written by a newer release; this release reads version {FORMAT_VERSION}"
                ),
            ),
            Some(v) => report.error("format_version", format!("unknown format version {v}")),
            None => report.error("format_version", "missing"),
        }
        if report.has_errors() {
            return Err(Error::Validation(report));
        }

        let repr: DocumentRepr = match serde_path_to_error::deserialize(value) {
            Ok(repr) => repr,
            Err(e) => {
                report.error(e.path().to_string(), e.inner().to_string());
                return Err(Error::Validation(report));
            }
        };
        Self::from_repr(repr, report)
    }

    fn from_repr(repr: DocumentRepr, mut report: ValidationReport) -> Result<PortfolioDocument> {
        report.merge("portfolio", repr.portfolio.validate());
        report.merge("", goal_names(&repr.goals));
        let market = repr.market.map(MarketRepr::into_market);
        report.into_result(PortfolioDocument {
            portfolio: repr.portfolio,
            market,
            goals: repr.goals,
        })
    }
}

/// The problems with the names of goals kept with a portfolio: each needs one, and no two the
/// same, so that each can be found by it.
pub fn goal_names(goals: &[PlanOptions]) -> ValidationReport {
    let mut report = ValidationReport::new();
    let mut seen = std::collections::BTreeSet::new();
    for (i, goal) in goals.iter().enumerate() {
        match goal.name.as_deref().map(str::trim) {
            None | Some("") => report.error(format!("goals[{i}].name"), "give the goal a name"),
            Some(name) => {
                if !seen.insert(name.to_owned()) {
                    report.error(
                        format!("goals[{i}].name"),
                        format!("another goal is named {name}"),
                    );
                }
            }
        }
    }
    report
}
