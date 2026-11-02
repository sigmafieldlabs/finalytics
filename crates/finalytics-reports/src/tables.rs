//! Reports as labelled tables, for export to CSV, spreadsheets or PDF.

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Decimal, Result};
use finalytics_ledger::HoldingKey;

use crate::context::{ReportKind, invalid, label, to_f64};
use crate::shapes::{Grid, Unit};
use crate::{
    AttributionReport, HoldingsReport, OverviewReport, PerformanceReport, RiskReport,
    ScenariosReport,
};

/// What a column holds, so that an export can format it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ColumnKind {
    /// Text.
    Text,
    /// An ISO 8601 date, as text.
    Date,
    /// A number.
    Number,
    /// A whole number.
    Integer,
    /// A fraction, to show as a percentage: 0.05 is 5%.
    Fraction,
    /// An amount of money in a currency.
    Money {
        /// The currency.
        currency: Currency,
    },
    /// Yes or no.
    Boolean,
}

/// One column of a [`ReportTable`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Column {
    /// A stable key for the column, such as `value_base`.
    pub key: String,
    /// A heading for people, such as "Value (USD)".
    pub label: String,
    /// What it holds.
    pub kind: ColumnKind,
}

/// One cell of a [`ReportTable`]: a number, text, yes or no, or nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum Cell {
    /// A number.
    Number(f64),
    /// Yes or no.
    Boolean(bool),
    /// Text, or a date as text.
    Text(String),
    /// Nothing.
    Missing,
}

/// A table of a report: named, with typed columns and one row of cells per record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ReportTable {
    /// A stable name, such as `holdings`, usable as a file or sheet name.
    pub name: String,
    /// A title for people, such as "Holdings".
    pub title: String,
    /// The columns.
    pub columns: Vec<Column>,
    /// The rows, one cell per column.
    pub rows: Vec<Vec<Cell>>,
}

/// Any report given as JSON, as the engine writes it, turned into its tables. The report's
/// provenance says which report it is.
pub fn report_tables(report: &str) -> Result<Vec<ReportTable>> {
    let value: serde_json::Value =
        serde_json::from_str(report).map_err(|e| invalid("report", format!("is not JSON: {e}")))?;
    let kind: ReportKind = serde_json::from_value(value["provenance"]["report"].clone())
        .map_err(|_| invalid("report.provenance.report", "is not a report kind"))?;
    fn read<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T> {
        serde_json::from_value(value).map_err(|e| invalid("report", e.to_string()))
    }
    Ok(match kind {
        ReportKind::Holdings => read::<HoldingsReport>(value)?.tables(),
        ReportKind::Overview => read::<OverviewReport>(value)?.tables(),
        ReportKind::Performance => read::<PerformanceReport>(value)?.tables(),
        ReportKind::Risk => read::<RiskReport>(value)?.tables(),
        ReportKind::Attribution => read::<AttributionReport>(value)?.tables(),
        ReportKind::Scenarios => read::<ScenariosReport>(value)?.tables(),
    })
}

fn n(x: f64) -> Cell {
    if x.is_finite() {
        Cell::Number(x)
    } else {
        Cell::Missing
    }
}

fn o(x: Option<f64>) -> Cell {
    x.map_or(Cell::Missing, n)
}

fn d(x: Decimal) -> Cell {
    n(to_f64(x))
}

fn od(x: Option<Decimal>) -> Cell {
    x.map_or(Cell::Missing, d)
}

fn t(x: impl Into<String>) -> Cell {
    Cell::Text(x.into())
}

fn date(x: Date) -> Cell {
    Cell::Text(x.to_string())
}

fn odate(x: Option<Date>) -> Cell {
    x.map_or(Cell::Missing, date)
}

fn int(x: impl Into<i64>) -> Cell {
    Cell::Number(x.into() as f64)
}

fn col(key: &str, label: &str, kind: ColumnKind) -> Column {
    Column {
        key: key.to_owned(),
        label: label.to_owned(),
        kind,
    }
}

fn money(currency: Currency) -> ColumnKind {
    ColumnKind::Money { currency }
}

fn table(name: &str, title: &str, columns: Vec<Column>, rows: Vec<Vec<Cell>>) -> ReportTable {
    ReportTable {
        name: name.to_owned(),
        title: title.to_owned(),
        columns,
        rows,
    }
}

fn kind_of(unit: Unit) -> ColumnKind {
    match unit {
        Unit::Money { currency } => ColumnKind::Money { currency },
        Unit::Fraction => ColumnKind::Fraction,
        Unit::Count => ColumnKind::Integer,
        Unit::Ratio | Unit::Level => ColumnKind::Number,
    }
}

/// A grid as a table: a first column of row labels, then one column per grid column.
fn grid_table(name: &str, first: &str, grid: &Grid) -> ReportTable {
    let mut columns = vec![col("row", first, ColumnKind::Text)];
    let kind = kind_of(grid.unit);
    for c in &grid.columns {
        columns.push(col(c, c, kind));
    }
    let rows = grid
        .rows
        .iter()
        .zip(&grid.values)
        .map(|(r, values)| {
            let mut row = vec![t(r.clone())];
            row.extend(values.iter().map(|v| o(*v)));
            row
        })
        .collect();
    table(name, &grid.name, columns, rows)
}

fn holding_name(key: &HoldingKey) -> Cell {
    t(label(key))
}

impl HoldingsReport {
    /// The report's tables: holdings, lots, allocation, realised gains and income.
    pub fn tables(&self) -> Vec<ReportTable> {
        let ccy = self.provenance.base_currency;
        let holdings = table(
            "holdings",
            "Holdings",
            vec![
                col("holding", "Holding", ColumnKind::Text),
                col("name", "Name", ColumnKind::Text),
                col("currency", "Currency", ColumnKind::Text),
                col("quantity", "Quantity", ColumnKind::Number),
                col("price", "Price", ColumnKind::Number),
                col("value_local", "Value (local)", ColumnKind::Number),
                col("value_base", &format!("Value ({ccy})"), money(ccy)),
                col("cost_base", &format!("Cost ({ccy})"), money(ccy)),
                col(
                    "unrealised_base",
                    &format!("Unrealised ({ccy})"),
                    money(ccy),
                ),
                col(
                    "unrealised_return",
                    "Unrealised return",
                    ColumnKind::Fraction,
                ),
                col("weight", "Weight", ColumnKind::Fraction),
            ],
            self.holdings
                .iter()
                .map(|h| {
                    vec![
                        holding_name(&h.holding),
                        h.name.clone().map_or(Cell::Missing, t),
                        t(h.currency.to_string()),
                        d(h.quantity),
                        od(h.price),
                        d(h.value_local),
                        d(h.value_base),
                        od(h.cost_base),
                        od(h.unrealised_base),
                        o(h.unrealised_return),
                        n(h.weight),
                    ]
                })
                .collect(),
        );
        let lots = table(
            "lots",
            "Tax lots",
            vec![
                col("account", "Account", ColumnKind::Text),
                col("instrument", "Instrument", ColumnKind::Text),
                col("lot", "Lot", ColumnKind::Text),
                col("acquired", "Acquired", ColumnKind::Date),
                col("quantity", "Quantity", ColumnKind::Number),
                col("cost_base", &format!("Cost ({ccy})"), money(ccy)),
                col("value_base", &format!("Value ({ccy})"), money(ccy)),
                col(
                    "unrealised_base",
                    &format!("Unrealised ({ccy})"),
                    money(ccy),
                ),
                col(
                    "unrealised_return",
                    "Unrealised return",
                    ColumnKind::Fraction,
                ),
                col("holding_days", "Days held", ColumnKind::Integer),
                col("long_term", "Long term", ColumnKind::Boolean),
            ],
            self.lots
                .iter()
                .map(|l| {
                    vec![
                        t(l.account.to_string()),
                        t(l.instrument.to_string()),
                        t(l.lot.to_string()),
                        date(l.acquired),
                        d(l.quantity),
                        d(l.cost_base),
                        d(l.value_base),
                        d(l.unrealised_base),
                        o(l.unrealised_return),
                        int(l.holding_days),
                        Cell::Boolean(l.long_term),
                    ]
                })
                .collect(),
        );
        let allocation = table(
            "allocation",
            "Allocation",
            vec![
                col("by", "By", ColumnKind::Text),
                col("name", "Name", ColumnKind::Text),
                col("value", &format!("Value ({ccy})"), money(ccy)),
                col("weight", "Weight", ColumnKind::Fraction),
            ],
            self.breakdowns
                .iter()
                .flat_map(|b| {
                    b.slices
                        .iter()
                        .map(|s| vec![t(b.by.clone()), t(s.name.clone()), n(s.value), n(s.weight)])
                })
                .collect(),
        );
        let realised = table(
            "realised_gains",
            "Realised gains by year",
            vec![
                col("year", "Year", ColumnKind::Integer),
                col("gain", &format!("Gain ({ccy})"), money(ccy)),
            ],
            self.realised
                .by_year
                .iter()
                .map(|(y, g)| vec![int(*y), d(*g)])
                .collect(),
        );
        let income = table(
            "income",
            "Income by year",
            vec![
                col("year", "Year", ColumnKind::Integer),
                col("income", &format!("Income ({ccy})"), money(ccy)),
            ],
            self.income
                .by_year
                .iter()
                .map(|(y, v)| vec![int(*y), d(*v)])
                .collect(),
        );
        vec![holdings, lots, allocation, realised, income]
    }
}

impl PerformanceReport {
    /// The report's tables: periods, monthly returns, calendar years, contributions, value and
    /// flows.
    pub fn tables(&self) -> Vec<ReportTable> {
        let ccy = self.provenance.base_currency;
        let periods = table(
            "periods",
            "Returns by period",
            vec![
                col("period", "Period", ColumnKind::Text),
                col("start", "Start", ColumnKind::Date),
                col("end", "End", ColumnKind::Date),
                col("cumulative", "Return", ColumnKind::Fraction),
                col("annualised", "Annualised", ColumnKind::Fraction),
                col("money_weighted", "Money-weighted", ColumnKind::Fraction),
                col("modified_dietz", "Modified Dietz", ColumnKind::Fraction),
                col("benchmark", "Benchmark", ColumnKind::Fraction),
                col("excess", "Excess", ColumnKind::Fraction),
            ],
            self.periods
                .iter()
                .map(|p| {
                    vec![
                        t(p.label.clone()),
                        odate(p.start),
                        odate(p.end),
                        o(p.cumulative),
                        o(p.annualised),
                        o(p.money_weighted),
                        o(p.modified_dietz),
                        o(p.benchmark),
                        o(p.excess_arithmetic),
                    ]
                })
                .collect(),
        );
        let years = table(
            "calendar_years",
            "Calendar-year returns",
            vec![
                col("year", "Year", ColumnKind::Integer),
                col("start", "From", ColumnKind::Date),
                col("end", "To", ColumnKind::Date),
                col("complete", "Whole year", ColumnKind::Boolean),
                col("portfolio", "Portfolio", ColumnKind::Fraction),
                col("benchmark", "Benchmark", ColumnKind::Fraction),
                col("excess", "Excess", ColumnKind::Fraction),
            ],
            self.calendar_years
                .iter()
                .map(|y| {
                    vec![
                        int(y.year),
                        date(y.start),
                        date(y.end),
                        Cell::Boolean(y.complete),
                        n(y.portfolio),
                        o(y.benchmark),
                        o(y.excess),
                    ]
                })
                .collect(),
        );
        let contributions = table(
            "contributions",
            "Contribution by holding",
            vec![
                col("holding", "Holding", ColumnKind::Text),
                col("contribution", "Contribution", ColumnKind::Fraction),
                col(
                    "return_base",
                    &format!("Return ({ccy})"),
                    ColumnKind::Fraction,
                ),
                col("return_local", "Return (local)", ColumnKind::Fraction),
                col("currency_effect", "Currency effect", ColumnKind::Fraction),
                col("average_weight", "Average weight", ColumnKind::Fraction),
            ],
            self.contributions
                .iter()
                .map(|c| {
                    vec![
                        holding_name(&c.holding),
                        n(c.contribution),
                        o(c.return_base),
                        o(c.return_local),
                        o(c.currency_effect),
                        n(c.average_weight),
                    ]
                })
                .collect(),
        );
        let mut value_columns = vec![
            col("date", "Date", ColumnKind::Date),
            col("value", &format!("Value ({ccy})"), money(ccy)),
            col("growth", "Growth of 100", ColumnKind::Number),
        ];
        if self.benchmark_growth.is_some() {
            value_columns.push(col(
                "benchmark",
                "Benchmark growth of 100",
                ColumnKind::Number,
            ));
        }
        let value = table(
            "value",
            "Value and growth",
            value_columns,
            self.value
                .dates
                .iter()
                .enumerate()
                .map(|(k, day)| {
                    let mut row = vec![
                        date(*day),
                        n(self.value.values[k]),
                        n(self.growth.values[k]),
                    ];
                    if let Some(b) = &self.benchmark_growth {
                        row.push(n(b.values[k]));
                    }
                    row
                })
                .collect(),
        );
        let flows = table(
            "flows",
            "Flows and income by month",
            vec![
                col("month", "Month", ColumnKind::Text),
                col("inflows", &format!("In ({ccy})"), money(ccy)),
                col("outflows", &format!("Out ({ccy})"), money(ccy)),
                col("income", &format!("Income ({ccy})"), money(ccy)),
            ],
            self.flows
                .iter()
                .map(|f| vec![t(f.month.clone()), n(f.inflows), n(f.outflows), n(f.income)])
                .collect(),
        );
        vec![
            periods,
            grid_table("monthly_returns", "Year", &self.monthly),
            years,
            contributions,
            value,
            flows,
        ]
    }
}

impl RiskReport {
    /// The report's tables: statistics, tail risk, drawdowns, risk by holding and correlation.
    pub fn tables(&self) -> Vec<ReportTable> {
        let ccy = self.provenance.base_currency;
        let s = &self.statistics;
        let mut measures: Vec<(&str, Cell)> = vec![
            ("Annualised return", n(s.annualised_return)),
            ("Volatility", n(s.volatility)),
            ("Downside deviation", n(s.downside_deviation)),
            ("Sharpe ratio", o(s.sharpe_ratio)),
            ("Sortino ratio", o(s.sortino_ratio)),
            ("Calmar ratio", o(s.calmar_ratio)),
            ("Maximum drawdown", n(s.max_drawdown)),
            ("Skewness", o(s.skewness)),
            ("Excess kurtosis", o(s.excess_kurtosis)),
            ("Best period", n(s.best)),
            ("Worst period", n(s.worst)),
        ];
        if let Some(r) = &self.relative {
            measures.extend([
                ("Active return", n(r.active_return)),
                ("Tracking error", n(r.tracking_error)),
                ("Information ratio", o(r.information_ratio)),
                ("Beta", o(r.beta)),
                ("Alpha", o(r.alpha)),
                ("Correlation", o(r.correlation)),
            ]);
        }
        let statistics = table(
            "statistics",
            "Statistics",
            vec![
                col("measure", "Measure", ColumnKind::Text),
                col("value", "Value", ColumnKind::Number),
            ],
            measures.into_iter().map(|(m, v)| vec![t(m), v]).collect(),
        );
        let tail = table(
            "tail_risk",
            "Value at risk and expected shortfall",
            vec![
                col("confidence", "Confidence", ColumnKind::Fraction),
                col("method", "Method", ColumnKind::Text),
                col("var", "Value at risk", ColumnKind::Fraction),
                col("var_amount", &format!("Value at risk ({ccy})"), money(ccy)),
                col("es", "Expected shortfall", ColumnKind::Fraction),
                col(
                    "es_amount",
                    &format!("Expected shortfall ({ccy})"),
                    money(ccy),
                ),
            ],
            self.tail
                .iter()
                .map(|r| {
                    let method = serde_json::to_value(r.method)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default();
                    vec![
                        n(r.confidence),
                        t(method),
                        n(r.value_at_risk.fraction),
                        n(r.value_at_risk.value),
                        n(r.expected_shortfall.fraction),
                        n(r.expected_shortfall.value),
                    ]
                })
                .collect(),
        );
        let drawdowns = table(
            "drawdowns",
            "Largest drawdowns",
            vec![
                col("peak", "Peak", ColumnKind::Date),
                col("trough", "Trough", ColumnKind::Date),
                col("recovery", "Recovery", ColumnKind::Date),
                col("depth", "Depth", ColumnKind::Fraction),
                col("days_to_trough", "Days to trough", ColumnKind::Integer),
                col("days_to_recovery", "Days to recovery", ColumnKind::Integer),
            ],
            self.drawdowns
                .iter()
                .map(|x| {
                    vec![
                        date(x.peak),
                        date(x.trough),
                        odate(x.recovery),
                        n(x.depth),
                        int(x.days_to_trough),
                        x.days_to_recovery.map_or(Cell::Missing, int),
                    ]
                })
                .collect(),
        );
        let mut out = vec![statistics, tail, drawdowns];
        if let Some(h) = &self.holdings {
            out.push(table(
                "risk_by_holding",
                "Risk by holding",
                vec![
                    col("holding", "Holding", ColumnKind::Text),
                    col("weight", "Weight", ColumnKind::Fraction),
                    col("marginal", "Marginal risk", ColumnKind::Fraction),
                    col("component", "Component risk", ColumnKind::Fraction),
                    col("share", "Share of risk", ColumnKind::Fraction),
                ],
                h.decomposition
                    .contributions
                    .iter()
                    .map(|c| {
                        vec![
                            t(c.label.clone()),
                            n(c.weight),
                            o(c.marginal),
                            n(c.component),
                            o(c.share),
                        ]
                    })
                    .collect(),
            ));
            out.push(grid_table("correlation", "Holding", &h.correlation));
        }
        out
    }
}

impl OverviewReport {
    /// The report's tables: headline figures, holdings' risk and return, and the frontier.
    pub fn tables(&self) -> Vec<ReportTable> {
        let ccy = self.provenance.base_currency;
        let h = &self.headline;
        let headline = table(
            "headline",
            "Overview",
            vec![
                col("measure", "Measure", ColumnKind::Text),
                col("value", "Value", ColumnKind::Number),
            ],
            vec![
                vec![t(format!("Value ({ccy})")), n(h.value)],
                vec![t("Return, last period"), o(h.period_return)],
                vec![t("Return, year to date"), o(h.year_to_date)],
                vec![t("Return since inception"), o(h.since_inception)],
                vec![
                    t("Annualised since inception"),
                    o(h.since_inception_annualised),
                ],
                vec![t("Volatility"), n(h.volatility)],
                vec![t("Sharpe ratio"), o(h.sharpe_ratio)],
                vec![t("Maximum drawdown"), n(h.max_drawdown)],
                vec![t("Value at risk"), o(h.value_at_risk.map(|a| a.fraction))],
                vec![
                    t("Expected shortfall"),
                    o(h.expected_shortfall.map(|a| a.fraction)),
                ],
            ],
        );
        let holdings = table(
            "holdings_risk_return",
            "Holdings: return and risk",
            vec![
                col("holding", "Holding", ColumnKind::Text),
                col("weight", "Weight", ColumnKind::Fraction),
                col("expected_return", "Expected return", ColumnKind::Fraction),
                col("given", "Given", ColumnKind::Boolean),
                col("volatility", "Volatility", ColumnKind::Fraction),
                col("risk_share", "Share of risk", ColumnKind::Fraction),
            ],
            self.holdings
                .iter()
                .map(|x| {
                    vec![
                        holding_name(&x.holding),
                        n(x.weight),
                        n(x.expected_return),
                        Cell::Boolean(x.expected_return_given),
                        n(x.volatility),
                        o(x.risk_share),
                    ]
                })
                .collect(),
        );
        let mut out = vec![headline, holdings];
        if let Some(points) = &self.frontier {
            out.push(table(
                "frontier",
                "Efficient frontier",
                vec![
                    col("expected_return", "Expected return", ColumnKind::Fraction),
                    col("volatility", "Volatility", ColumnKind::Fraction),
                ],
                points
                    .iter()
                    .map(|p| vec![n(p.expected_return), n(p.volatility)])
                    .collect(),
            ));
        }
        out
    }
}

impl AttributionReport {
    /// The report's tables: effects by period and segment, and linked by segment.
    pub fn tables(&self) -> Vec<ReportTable> {
        let effects = vec![
            col("allocation", "Allocation", ColumnKind::Fraction),
            col("selection", "Selection", ColumnKind::Fraction),
            col("interaction", "Interaction", ColumnKind::Fraction),
            col("total", "Total", ColumnKind::Fraction),
        ];
        let mut columns = vec![
            col("start", "Start", ColumnKind::Date),
            col("end", "End", ColumnKind::Date),
            col("segment", "Segment", ColumnKind::Text),
            col("portfolio_weight", "Portfolio weight", ColumnKind::Fraction),
            col("portfolio_return", "Portfolio return", ColumnKind::Fraction),
            col("benchmark_weight", "Benchmark weight", ColumnKind::Fraction),
            col("benchmark_return", "Benchmark return", ColumnKind::Fraction),
        ];
        columns.extend(effects.iter().cloned());
        let mut rows = Vec::new();
        for (p, a) in self.periods.iter().zip(&self.attribution.periods) {
            for (s, e) in p.segments.iter().zip(&a.segments) {
                rows.push(vec![
                    date(p.start),
                    date(p.end),
                    t(s.name.clone()),
                    n(s.portfolio_weight),
                    n(s.portfolio_return),
                    n(s.benchmark_weight),
                    n(s.benchmark_return),
                    n(e.allocation),
                    n(e.selection),
                    n(e.interaction),
                    n(e.total),
                ]);
            }
        }
        let by_period = table(
            "attribution_by_period",
            "Attribution by period",
            columns,
            rows,
        );
        let mut columns = vec![col("segment", "Segment", ColumnKind::Text)];
        columns.extend(effects);
        let a = &self.attribution;
        let mut rows: Vec<Vec<Cell>> = a
            .segments
            .iter()
            .map(|e| {
                vec![
                    t(e.name.clone()),
                    n(e.allocation),
                    n(e.selection),
                    n(e.interaction),
                    n(e.total),
                ]
            })
            .collect();
        rows.push(vec![
            t("Total"),
            n(a.allocation),
            n(a.selection),
            n(a.interaction),
            n(a.excess_return),
        ]);
        vec![
            by_period,
            table("attribution", "Attribution, linked", columns, rows),
        ]
    }
}

impl ScenariosReport {
    /// The report's tables: the summary, and each holding's change in each scenario.
    pub fn tables(&self) -> Vec<ReportTable> {
        let ccy = self.provenance.base_currency;
        let summary = table(
            "scenarios",
            "Scenarios",
            vec![
                col("name", "Scenario", ColumnKind::Text),
                col("change", &format!("Change ({ccy})"), money(ccy)),
                col("return", "Return", ColumnKind::Fraction),
                col("volatility_after", "Volatility after", ColumnKind::Fraction),
                col(
                    "var_after",
                    &format!("Value at risk after ({ccy})"),
                    money(ccy),
                ),
                col("worst", "Worst holding", ColumnKind::Text),
                col("worst_change", &format!("Its change ({ccy})"), money(ccy)),
            ],
            self.summary
                .iter()
                .map(|s| {
                    vec![
                        t(s.name.clone()),
                        n(s.change),
                        n(s.portfolio_return),
                        o(s.volatility_after),
                        o(s.value_at_risk_after),
                        s.worst.as_ref().map_or(Cell::Missing, holding_name),
                        o(s.worst_change),
                    ]
                })
                .collect(),
        );
        vec![
            summary,
            grid_table("scenario_changes", "Holding", &self.changes),
        ]
    }
}
