//! Writes the JSON Schema of the Finalytics data types into `schema/`.
//!
//! ```sh
//! cargo run -p finalytics-codegen           # write the schema files
//! cargo run -p finalytics-codegen -- --check  # fail if they are out of date
//! ```
//!
//! `finalytics.schema.json` describes every type the Python and JavaScript bindings exchange
//! with the engine; the bindings' type declarations are generated from it by
//! `scripts/generate-types.py`. `portfolio-document.schema.json` describes the portfolio
//! document on its own, for applications that read or write it.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

use finalytics::{formats::PortfolioDocument, opt, risk};

const BASE_URL: &str = "https://github.com/sigmafieldlabs/finalytics/blob/main/schema";

fn generator() -> SchemaGenerator {
    SchemaSettings::draft2020_12().into_generator()
}

/// Adds `T` to the definitions.
fn add<T: JsonSchema>(generator: &mut SchemaGenerator) {
    generator.subschema_for::<T>();
}

/// Every type the bindings exchange with the engine.
fn api() -> Value {
    let mut g = generator();
    // Foundations and the ledger.
    add::<finalytics::ValidationReport>(&mut g);
    add::<finalytics::Money>(&mut g);
    add::<finalytics::PortfolioData>(&mut g);
    add::<PortfolioDocument>(&mut g);
    add::<finalytics::Position>(&mut g);
    add::<finalytics::Lot>(&mut g);
    add::<finalytics::CashBalance>(&mut g);
    add::<finalytics::RealisedGain>(&mut g);
    add::<finalytics::Event>(&mut g);
    add::<finalytics::Valuation>(&mut g);
    add::<finalytics::ValueSeries>(&mut g);
    add::<finalytics::HoldingSeries>(&mut g);
    add::<finalytics::LotSale>(&mut g);
    // Performance.
    add::<finalytics::performance::FlowTiming>(&mut g);
    add::<finalytics::performance::Period>(&mut g);
    add::<finalytics::performance::PeriodReturn>(&mut g);
    add::<finalytics::performance::ExcessReturn>(&mut g);
    add::<finalytics::performance::Contribution>(&mut g);
    add::<finalytics::performance::GroupContribution>(&mut g);
    add::<finalytics::performance::AttributionPeriod>(&mut g);
    add::<finalytics::performance::Attribution>(&mut g);
    add::<finalytics::performance::CurrencyAttributionPeriod>(&mut g);
    add::<finalytics::performance::CurrencyAttribution>(&mut g);
    add::<finalytics::risk::Shocks>(&mut g);
    add::<finalytics::risk::HistoricalWindow>(&mut g);
    add::<finalytics::risk::ScenarioOptions>(&mut g);
    add::<finalytics::risk::CorrelationStress>(&mut g);
    add::<finalytics::risk::ScenarioResult>(&mut g);
    add::<finalytics::AccountSeries>(&mut g);
    add::<finalytics::performance::HouseholdOptions>(&mut g);
    add::<finalytics::performance::Household>(&mut g);
    add::<finalytics::reports::ReportOptions>(&mut g);
    add::<finalytics::reports::Source>(&mut g);
    add::<finalytics::reports::HoldingsReport>(&mut g);
    add::<finalytics::reports::OverviewReport>(&mut g);
    add::<finalytics::reports::PerformanceReport>(&mut g);
    add::<finalytics::reports::RiskReport>(&mut g);
    add::<finalytics::reports::AttributionReport>(&mut g);
    add::<finalytics::reports::ScenariosReport>(&mut g);
    add::<finalytics::reports::ReportTable>(&mut g);
    // Risk.
    add::<risk::StatisticsOptions>(&mut g);
    add::<risk::Statistics>(&mut g);
    add::<risk::RelativeStatistics>(&mut g);
    add::<risk::TailMethod>(&mut g);
    add::<risk::TailRisk>(&mut g);
    add::<risk::Drawdown>(&mut g);
    add::<risk::CovarianceMethod>(&mut g);
    add::<risk::CovarianceEstimate>(&mut g);
    add::<risk::RiskDecomposition>(&mut g);
    add::<risk::GroupRisk>(&mut g);
    add::<risk::HoldingReturns>(&mut g);
    add::<risk::FactorModel>(&mut g);
    add::<risk::FactorModelEstimate>(&mut g);
    add::<risk::FactorRisk>(&mut g);
    // Optimisation.
    add::<opt::Problem>(&mut g);
    add::<opt::Objective>(&mut g);
    add::<opt::Solution>(&mut g);
    add::<opt::FrontierPoint>(&mut g);
    add::<opt::RebalanceOptions>(&mut g);
    add::<opt::Rebalance>(&mut g);
    add::<opt::RebalanceResults>(&mut g);
    add::<opt::BlackLitterman>(&mut g);
    add::<opt::BlackLittermanEstimate>(&mut g);
    add::<opt::Resampling>(&mut g);
    add::<opt::ResampledSolution>(&mut g);
    // The front doors.
    add::<finalytics::PriceTable>(&mut g);
    add::<finalytics::AssetStatistics>(&mut g);
    add::<finalytics::OptimiseOptions>(&mut g);
    add::<finalytics::OptimisationResult>(&mut g);
    add::<finalytics::PortfolioReport>(&mut g);
    add::<finalytics::datasets::Company>(&mut g);
    // Backtests.
    add::<finalytics::backtest::BacktestOptions>(&mut g);
    add::<finalytics::Optimisation>(&mut g);
    add::<finalytics::Comparison>(&mut g);
    add::<finalytics::Backtest>(&mut g);
    add::<finalytics::OptimisationReport>(&mut g);
    add::<finalytics::ComparisonReport>(&mut g);
    add::<finalytics::BacktestReport>(&mut g);
    add::<finalytics::backtest::BacktestResults>(&mut g);
    // Rebalancing.
    add::<finalytics::Rebalancing>(&mut g);
    add::<finalytics::RebalanceReport>(&mut g);
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{BASE_URL}/finalytics.schema.json"),
        "title": "Finalytics data types",
        "description": "The types exchanged between the Finalytics engine and its Python and JavaScript bindings.",
        "$defs": with_decimal(g.take_definitions(true)),
    })
}

fn with_decimal(mut definitions: serde_json::Map<String, Value>) -> Value {
    let mut value = Value::Object(std::mem::take(&mut definitions));
    tidy(&mut value);
    if let Value::Object(map) = &mut value {
        map.insert("Decimal".into(), decimal());
        map.sort_keys();
    }
    value
}

/// The portfolio document on its own.
fn document() -> Value {
    let mut schema: Schema = generator().into_root_schema_for::<PortfolioDocument>();
    schema.insert(
        "$id".into(),
        format!("{BASE_URL}/portfolio-document.schema.json").into(),
    );
    schema.insert("title".into(), "Finalytics portfolio document".into());
    let mut value = schema.to_value();
    tidy(&mut value);
    if let Some(Value::Object(defs)) = value.get_mut("$defs") {
        defs.insert("Decimal".into(), decimal());
        defs.sort_keys();
    }
    value
}

/// Whether a schema is the one schemars gives `rust_decimal::Decimal`, which this tool names
/// `Decimal`, and whether it also allows null (for `Option<Decimal>`).
fn decimal_kind(map: &serde_json::Map<String, Value>) -> Option<bool> {
    if map.get("pattern").and_then(Value::as_str) != Some(r"^-?\d+(\.\d+)?([eE]\d+)?$") {
        return None;
    }
    match map.get("type") {
        Some(t) if t == &json!(["string", "number"]) => Some(false),
        Some(t) if t == &json!(["string", "number", "null"]) => Some(true),
        _ => None,
    }
}

/// Tidies the generated schema: decimals refer to one `Decimal` definition, and descriptions keep
/// their prose but lose Rust code examples and link brackets.
fn tidy(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(nullable) = decimal_kind(map) {
                map.remove("pattern");
                map.remove("type");
                if nullable {
                    map.insert(
                        "anyOf".into(),
                        json!([{"$ref": "#/$defs/Decimal"}, {"type": "null"}]),
                    );
                } else {
                    map.insert("$ref".into(), "#/$defs/Decimal".into());
                }
            }
            if let Some(Value::String(text)) = map.get_mut("description") {
                *text = prose(text);
            }
            map.values_mut().for_each(tidy);
        }
        Value::Array(items) => items.iter_mut().for_each(tidy),
        _ => {}
    }
}

/// A doc comment without fenced code blocks, and with rustdoc links (`` [`Item`] ``) as code.
fn prose(text: &str) -> String {
    let mut out = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            out.push(line);
        }
    }
    let joined = out.join("\n").trim().to_owned();
    joined.replace("[`", "`").replace("`]", "`")
}

fn decimal() -> Value {
    json!({
        "description": "An exact decimal number. Finalytics writes decimals as strings, such as \"201.50\", and reads strings or numbers.",
        "type": ["string", "number"],
        "pattern": r"^-?\d+(\.\d+)?([eE]\d+)?$"
    })
}

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema");
    let files: [(PathBuf, Value); 2] = [
        (root.join("finalytics.schema.json"), api()),
        (root.join("portfolio-document.schema.json"), document()),
    ];
    let mut stale = false;
    for (path, value) in files {
        let text = serde_json::to_string_pretty(&value).unwrap_or_default() + "\n";
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if check {
            if current != text {
                eprintln!(
                    "{} is out of date; run `cargo run -p finalytics-codegen`",
                    path.display()
                );
                stale = true;
            }
        } else if current != text {
            if let Err(e) =
                std::fs::create_dir_all(&root).and_then(|()| std::fs::write(&path, text))
            {
                eprintln!("could not write {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {}", path.display());
        }
    }
    if stale {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
